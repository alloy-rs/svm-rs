use crate::{
    all_releases, data_dir, platform, releases::artifact_url, setup_data_dir, setup_version,
    version_binary, SvmError,
};
use semver::Version;
use sha2::Digest;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tempfile::NamedTempFile;

#[cfg(target_family = "unix")]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

/// The timeout to use for requests to the source (10 minutes).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

/// Version beyond which solc binaries are not fully static, hence need to be patched for NixOS.
const NIXOS_MIN_PATCH_VERSION: Version = Version::new(0, 7, 6);

/// Blocking version of [`install`]
#[cfg(feature = "blocking")]
pub fn blocking_install(version: &Version) -> Result<PathBuf, SvmError> {
    setup_data_dir()?;

    let artifacts = crate::blocking_all_releases(platform::platform())?;
    let artifact = artifacts
        .get_artifact(version)
        .ok_or(SvmError::UnknownVersion)?;
    let download_url = artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let expected_checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    let res = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("reqwest::Client::new()")
        .get(download_url.clone())
        .send()?;

    if !res.status().is_success() {
        return Err(SvmError::UnsuccessfulResponse(download_url, res.status()));
    }

    let binbytes = res.bytes()?;
    ensure_checksum(&binbytes, version, &expected_checksum)?;

    // lock file to indicate that installation of this solc version will be in progress.
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of solc.
    let _lock = try_lock_file(lock_path)?;

    do_install(version, &binbytes, artifact.to_string().as_str())
}

/// Installs the provided version of Solc in the machine.
///
/// Returns the path to the solc file.
pub async fn install(version: &Version) -> Result<PathBuf, SvmError> {
    setup_data_dir()?;

    let artifacts = all_releases(platform::platform()).await?;
    let artifact = artifacts
        .releases
        .get(version)
        .ok_or(SvmError::UnknownVersion)?;
    let download_url = artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let expected_checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    let res = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("reqwest::Client::new()")
        .get(download_url.clone())
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(SvmError::UnsuccessfulResponse(download_url, res.status()));
    }

    let binbytes = res.bytes().await?;
    ensure_checksum(&binbytes, version, &expected_checksum)?;

    // lock file to indicate that installation of this solc version will be in progress.
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of solc.
    let _lock = try_lock_file(lock_path)?;

    do_install(version, &binbytes, artifact.to_string().as_str())
}

fn do_install(version: &Version, binbytes: &[u8], _artifact: &str) -> Result<PathBuf, SvmError> {
    setup_version(&version.to_string())?;
    let installer = Installer { version, binbytes };

    // Solc versions <= 0.7.1 are .zip files for Windows only
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    if _artifact.ends_with(".zip") {
        return installer.install_zip();
    }

    installer.install()
}

/// Creates the file and locks it exclusively, this will block if the file is currently locked
fn try_lock_file(lock_path: PathBuf) -> Result<LockFile, SvmError> {
    use fs4::fs_std::FileExt;
    let _lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    _lock_file.lock_exclusive()?;
    Ok(LockFile {
        lock_path,
        _lock_file,
    })
}

/// Represents a lockfile that's removed once dropped
struct LockFile {
    _lock_file: fs::File,
    lock_path: PathBuf,
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Returns the lockfile to use for a specific file
fn lock_file_path(version: &Version) -> PathBuf {
    data_dir().join(format!(".lock-solc-{version}"))
}

// Installer type that copies binary data to the appropriate solc binary file:
// 1. create target file to copy binary data
// 2. copy data
struct Installer<'a> {
    // version of solc
    version: &'a Version,
    // binary data of the solc executable
    binbytes: &'a [u8],
}

impl Installer<'_> {
    /// Installs the solc version at the version specific destination and returns the path to the installed solc file.
    fn install(self) -> Result<PathBuf, SvmError> {
        let named_temp_file = NamedTempFile::new_in(data_dir())?;
        let (mut f, temp_path) = named_temp_file.into_parts();

        #[cfg(target_family = "unix")]
        f.set_permissions(Permissions::from_mode(0o755))?;
        f.write_all(self.binbytes)?;

        if platform::is_nixos() && *self.version >= NIXOS_MIN_PATCH_VERSION {
            patch_for_nixos(&temp_path)?;
        }

        let solc_path = version_binary(&self.version.to_string());

        // Windows requires that the old file be moved out of the way first.
        if cfg!(target_os = "windows") {
            let temp_path = NamedTempFile::new_in(data_dir()).map(NamedTempFile::into_temp_path)?;
            fs::rename(&solc_path, &temp_path).unwrap_or_default();
        }

        temp_path.persist(&solc_path)?;

        Ok(solc_path)
    }

    /// Extracts the solc archive at the version specified destination and returns the path to the
    /// installed solc binary.
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn install_zip(self) -> Result<PathBuf, SvmError> {
        let solc_path = version_binary(&self.version.to_string());
        let version_path = solc_path.parent().unwrap();

        let mut content = std::io::Cursor::new(self.binbytes);
        let mut archive = zip::ZipArchive::new(&mut content)?;
        archive.extract(version_path)?;

        std::fs::rename(version_path.join("solc.exe"), &solc_path)?;

        Ok(solc_path)
    }
}

/// Patch the given binary to use the dynamic linker provided by nixos.
fn patch_for_nixos(bin: &Path) -> Result<(), SvmError> {
    let output = Command::new("nix-shell")
        .arg("-p")
        .arg("patchelf")
        .arg("--run")
        .arg(format!(
            "patchelf --set-interpreter \"$(cat $NIX_CC/nix-support/dynamic-linker)\" {}",
            bin.display()
        ))
        .output()
        .expect("Failed to execute command");

    match output.status.success() {
        true => Ok(()),
        false => Err(SvmError::CouldNotPatchForNixOs(
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )),
    }
}

fn ensure_checksum(
    binbytes: &[u8],
    version: &Version,
    expected_checksum: &[u8],
) -> Result<(), SvmError> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(binbytes);
    let checksum = &hasher.finalize()[..];
    // checksum does not match
    if checksum != expected_checksum {
        return Err(SvmError::ChecksumMismatch {
            version: version.to_string(),
            expected: hex::encode(expected_checksum),
            actual: hex::encode(checksum),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::SliceRandom;

    #[allow(unused)]
    const LATEST: Version = Version::new(0, 8, 27);

    #[tokio::test]
    #[serial_test::serial]
    async fn test_install() {
        let versions = all_releases(platform())
            .await
            .unwrap()
            .releases
            .into_keys()
            .collect::<Vec<Version>>();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(install(rand_version).await.is_ok());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn can_install_while_solc_is_running() {
        const WHICH: &str = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };

        let version: Version = "0.8.10".parse().unwrap();
        let solc_path = version_binary(version.to_string().as_str());

        fs::create_dir_all(solc_path.parent().unwrap()).unwrap();

        // Overwrite solc with `sleep` and call it with `infinity`.
        let stdout = Command::new(WHICH).arg("sleep").output().unwrap().stdout;
        let sleep_path = String::from_utf8(stdout).unwrap();
        fs::copy(sleep_path.trim_end(), &solc_path).unwrap();
        let mut child = Command::new(solc_path).arg("infinity").spawn().unwrap();

        // Install should not fail with "text file busy".
        install(&version).await.unwrap();

        child.kill().unwrap();
        let _: std::process::ExitStatus = child.wait().unwrap();
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_install() {
        let versions = crate::releases::blocking_all_releases(platform::platform())
            .unwrap()
            .into_versions();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(blocking_install(rand_version).is_ok());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_version() {
        let version = "0.8.10".parse().unwrap();
        install(&version).await.unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        let output = Command::new(solc_path).arg("--version").output().unwrap();
        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.8.10"));
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_latest() {
        blocking_install(&LATEST).unwrap();
        let solc_path = version_binary(LATEST.to_string().as_str());
        let output = Command::new(solc_path).arg("--version").output().unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains(&LATEST.to_string()));
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_version() {
        let version = "0.8.10".parse().unwrap();
        blocking_install(&version).unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        let output = Command::new(solc_path).arg("--version").output().unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.8.10"));
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn can_install_parallel() {
        let version: Version = "0.8.10".parse().unwrap();
        let cloned_version = version.clone();
        let t = std::thread::spawn(move || blocking_install(&cloned_version));
        blocking_install(&version).unwrap();
        t.join().unwrap().unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_install_parallel_async() {
        let version: Version = "0.8.10".parse().unwrap();
        let cloned_version = version.clone();
        let t = tokio::task::spawn(async move { install(&cloned_version).await });
        install(&version).await.unwrap();
        t.await.unwrap().unwrap();
    }

    // ensures we can download the latest universal solc for apple silicon
    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    async fn can_install_latest_native_apple_silicon() {
        let solc = install(&LATEST).await.unwrap();
        let output = Command::new(solc).arg("--version").output().unwrap();
        let version = String::from_utf8_lossy(&output.stdout);
        assert!(version.contains("0.8.27"), "{}", version);
    }

    // ensures we can download the latest native solc for linux aarch64
    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    async fn can_download_latest_linux_aarch64() {
        let artifacts = all_releases(Platform::LinuxAarch64).await.unwrap();

        let artifact = artifacts.releases.get(&LATEST).unwrap();
        let download_url = artifact_url(
            Platform::LinuxAarch64,
            &LATEST,
            artifact.to_string().as_str(),
        )
        .unwrap();

        let checksum = artifacts.get_checksum(&LATEST).unwrap();

        let resp = reqwest::get(download_url).await.unwrap();
        assert!(resp.status().is_success());
        let binbytes = resp.bytes().await.unwrap();
        ensure_checksum(&binbytes, &LATEST, checksum).unwrap();
    }

    #[tokio::test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    async fn can_install_windows_zip_release() {
        let version = "0.7.1".parse().unwrap();
        install(&version).await.unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        let output = Command::new(&solc_path).arg("--version").output().unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.7.1"));
    }
}
