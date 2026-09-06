use crate::{
    SvmError, all_releases, data_dir, platform, releases::artifact_url, setup_data_dir,
    setup_version, version_binary, version_path,
};
use semver::Version;
use sha2::Digest;
use std::{
    fs,
    io::{ErrorKind, Write},
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

/// Version beyond which solc binaries are fully static again, hence no patching is needed for NixOS.
/// See <https://github.com/ethereum/solidity/releases/tag/v0.8.29>
const NIXOS_MAX_PATCH_VERSION: Version = Version::new(0, 8, 28);

/// Blocking version of [`install`]
#[cfg(feature = "blocking")]
pub fn blocking_install(version: &Version) -> Result<PathBuf, SvmError> {
    setup_data_dir()?;

    let artifacts = crate::blocking_all_releases(platform::platform())?;
    let artifact = artifacts
        .get_artifact(version)
        .ok_or_else(|| SvmError::UnknownVersion(version.clone()))?;
    let download_url = artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let expected_checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    // Skip the download if this version is already installed and matches the expected checksum,
    // which is the common case when parallel processes install the same version.
    if let Some(solc_path) = find_reusable_installation(version, &expected_checksum) {
        return Ok(solc_path);
    }

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
    setup_version(&version.to_string())?;
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of solc.
    let _lock = try_lock_file(&lock_path)?;

    do_install_and_retry(
        version,
        &binbytes,
        artifact.to_string().as_str(),
        &expected_checksum,
    )
}

/// Installs the provided version of Solc in the machine.
///
/// If the version is already installed and matches the expected checksum, it is reused instead of
/// being reinstalled.
///
/// Returns the path to the solc file.
pub async fn install(version: &Version) -> Result<PathBuf, SvmError> {
    setup_data_dir()?;

    let artifacts = all_releases(platform::platform()).await?;
    let artifact = artifacts
        .get_artifact(version)
        .ok_or_else(|| SvmError::UnknownVersion(version.clone()))?;
    let download_url = artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let expected_checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    // Skip the download if this version is already installed and matches the expected checksum,
    // which is the common case when parallel processes install the same version.
    if let Some(solc_path) = find_reusable_installation(version, &expected_checksum) {
        return Ok(solc_path);
    }

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
    setup_version(&version.to_string())?;
    let lock_path = lock_file_path(version);
    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of solc.
    let _lock = try_lock_file(&lock_path)?;

    do_install_and_retry(
        version,
        &binbytes,
        artifact.to_string().as_str(),
        &expected_checksum,
    )
}

/// Returns the path of the solc binary if `version` is already installed and its contents match
/// the expected artifact checksum.
///
/// Note: on NixOS the installed binary is patched with `patchelf` and for old solc versions on
/// Windows the artifact is a zip archive, so in both cases the file on disk never matches the
/// artifact checksum and the installation is never considered reusable here.
fn find_reusable_installation(version: &Version, expected_checksum: &[u8]) -> Option<PathBuf> {
    let solc_path = version_binary(&version.to_string());
    if let Ok(content) = fs::read(&solc_path)
        && ensure_checksum(&content, version, expected_checksum).is_ok()
    {
        // checksum of the existing file matches the expected release checksum
        return Some(solc_path);
    }
    None
}

/// Same as [`do_install`] but reuses an already installed binary if it matches the expected
/// checksum and retries "text file busy" errors.
///
/// Expects the per-version lock to be held by the caller.
fn do_install_and_retry(
    version: &Version,
    binbytes: &[u8],
    artifact: &str,
    expected_checksum: &[u8],
) -> Result<PathBuf, SvmError> {
    let mut retries = 0;

    loop {
        // A parallel process may have already installed this version while we were downloading or
        // waiting for the lock. In that case reuse the existing binary instead of replacing it,
        // because it can already be executing in another process.
        if let Some(solc_path) = find_reusable_installation(version, expected_checksum) {
            return Ok(solc_path);
        }

        return match do_install(version, binbytes, artifact) {
            Ok(path) => Ok(path),
            Err(err) => {
                // installation failed
                if retries > 2 {
                    return Err(err);
                }
                retries += 1;
                // check if this failed due to a text file busy, which indicates that a different process started using the target file
                if err.to_string().to_lowercase().contains("text file busy") {
                    // busy solc can be in use for a while (e.g. if compiling a large project), so
                    // we retry after some time; the loop re-checks whether a valid binary exists
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }

                Err(err)
            }
        };
    }
}

fn do_install(version: &Version, binbytes: &[u8], _artifact: &str) -> Result<PathBuf, SvmError> {
    setup_version(&version.to_string())?;
    let installer = Installer { version, binbytes };

    // Solc versions <= 0.7.1 are .zip files for Windows only
    #[cfg(target_os = "windows")]
    if _artifact.ends_with(".zip") {
        return installer.install_zip();
    }

    installer.install()
}

/// Creates or opens the lock file and locks it exclusively, this will block if the file is
/// currently locked by another process.
///
/// The lock is released once the returned file is dropped.
///
/// Note: the lock file is intentionally never removed. Removing it while another process is
/// blocked on the same path would let a third process re-create the path as a new file and lock
/// it immediately, so that two processes hold the "exclusive" lock at the same time.
fn try_lock_file(lock_path: &Path) -> Result<fs::File, SvmError> {
    loop {
        let lock_file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)?;
        lock_file.lock()?;

        // The lock file may have been removed and re-created while we were blocked on the lock,
        // e.g. by `remove_version` removing the version directory: in that case the acquired lock
        // is held on an orphaned file, so retry on the file that now exists at the path.
        if lock_is_current(&lock_file, lock_path)? {
            return Ok(lock_file);
        }
    }
}

/// Returns whether the locked file is still the file at `lock_path`.
#[cfg(target_family = "unix")]
fn lock_is_current(lock_file: &fs::File, lock_path: &Path) -> Result<bool, SvmError> {
    use std::os::unix::fs::MetadataExt;
    let held = lock_file.metadata()?;
    match fs::metadata(lock_path) {
        Ok(current) => Ok(current.dev() == held.dev() && current.ino() == held.ino()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err.into()),
    }
}

/// Returns whether the locked file is still the file at `lock_path`.
#[cfg(not(target_family = "unix"))]
fn lock_is_current(_lock_file: &fs::File, _lock_path: &Path) -> Result<bool, SvmError> {
    // On Windows a locked file cannot be removed, so the lock is always held on the current file.
    Ok(true)
}

/// Returns the lockfile to use for a specific version.
///
/// The lock file lives inside the version directory so that the data directory root only ever
/// contains version directories and the global version file: svm-rs <= 0.5.26 fails to list
/// installed versions if the data directory root contains any other entry.
fn lock_file_path(version: &Version) -> PathBuf {
    version_path(&version.to_string()).join(".lock")
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
        let version = self.version.to_string();
        let version_dir = version_path(&version);
        let solc_path = version_binary(&version);

        // The temp file lives inside the version directory so that the data directory root only
        // ever contains version directories and the global version file, see [`lock_file_path`].
        let named_temp_file = NamedTempFile::new_in(&version_dir)?;
        let (mut f, temp_path) = named_temp_file.into_parts();

        #[cfg(target_family = "unix")]
        f.set_permissions(Permissions::from_mode(0o755))?;
        f.write_all(self.binbytes)?;
        f.sync_data()?;
        // Close the file before renaming it into place: once the rename makes it reachable at the
        // solc path, an open write handle would cause "text file busy" (`ETXTBSY`) errors for any
        // process that tries to execute the binary.
        drop(f);

        if platform::is_nixos()
            && *self.version >= NIXOS_MIN_PATCH_VERSION
            && *self.version <= NIXOS_MAX_PATCH_VERSION
        {
            patch_for_nixos(self.version, &temp_path)?;
        }

        // Windows requires that the old file be moved out of the way first.
        if cfg!(target_os = "windows") {
            let temp_path =
                NamedTempFile::new_in(&version_dir).map(NamedTempFile::into_temp_path)?;
            fs::rename(&solc_path, &temp_path).unwrap_or_default();
        }

        temp_path.persist(&solc_path)?;

        Ok(solc_path)
    }

    /// Extracts the solc archive at the version specified destination and returns the path to the
    /// installed solc binary.
    #[cfg(target_os = "windows")]
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
fn patch_for_nixos(version: &Version, bin: &Path) -> Result<(), SvmError> {
    let dynamic_linker = nixos_dynamic_linker()?;
    add_gc_root_for_store_path(version, &dynamic_linker)?;

    let output = Command::new("nix-shell")
        .arg("-p")
        .arg("patchelf")
        .arg("--run")
        .arg(format!(
            "patchelf --set-interpreter \"{}\" {}",
            dynamic_linker,
            bin.display()
        ))
        .output()
        .map_err(|e| SvmError::CouldNotPatchForNixOs(String::new(), e.to_string()))?;

    match output.status.success() {
        true => Ok(()),
        false => Err(SvmError::CouldNotPatchForNixOs(
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )),
    }
}

/// Resolves the NixOS dynamic linker path from the nix-shell environment.
fn nixos_dynamic_linker() -> Result<String, SvmError> {
    let output = Command::new("nix-shell")
        .arg("-p")
        .arg("patchelf")
        .arg("--run")
        .arg("cat $NIX_CC/nix-support/dynamic-linker")
        .output()
        .map_err(|e| SvmError::CouldNotPatchForNixOs(String::new(), e.to_string()))?;

    if !output.status.success() {
        return Err(SvmError::CouldNotPatchForNixOs(
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    let dynamic_linker = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if dynamic_linker.is_empty() {
        return Err(SvmError::CouldNotPatchForNixOs(
            String::new(),
            "empty dynamic linker path from nix-shell".to_string(),
        ));
    }

    Ok(dynamic_linker)
}

/// Adds a persistent gcroot for a nix store path used by a specific installed solc version.
fn add_gc_root_for_store_path(version: &Version, store_path: &str) -> Result<(), SvmError> {
    let gcroots_dir = data_dir().join(".gcroots");
    fs::create_dir_all(&gcroots_dir)?;

    // One gcroot per solc version to avoid repointing a shared root when linker paths change.
    let root_path = gcroots_dir.join(format!("solc-{version}-dynamic-linker"));

    match fs::remove_file(&root_path) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }

    let output = Command::new("nix-store")
        .arg("--add-root")
        .arg(&root_path)
        .arg("--realise")
        .arg(store_path)
        .output()
        .map_err(|e| SvmError::CouldNotAddNixGcRoot(String::new(), e.to_string()))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(SvmError::CouldNotAddNixGcRoot(
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
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
    use rand::seq::IndexedRandom;

    #[allow(unused)]
    const LATEST: Version = Version::new(0, 8, 36);

    #[tokio::test]
    #[serial_test::serial]
    async fn test_install() {
        let versions = all_releases(platform())
            .await
            .unwrap()
            .releases
            .into_keys()
            .collect::<Vec<Version>>();
        let rand_version = versions.choose(&mut rand::rng()).unwrap();
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
        // Long-running command: `sleep infinity` on Unix, `timeout /t 3600` on Windows
        const CMD: &str = if cfg!(target_os = "windows") {
            "timeout"
        } else {
            "sleep"
        };

        let version: Version = "0.8.10".parse().unwrap();
        let solc_path = version_binary(version.to_string().as_str());

        fs::create_dir_all(solc_path.parent().unwrap()).unwrap();

        // Overwrite solc with a long-running command.
        let stdout = Command::new(WHICH).arg(CMD).output().unwrap().stdout;
        let cmd_path = String::from_utf8(stdout).unwrap();
        // On Windows, `where` can return multiple paths - take the first one
        let cmd_path = cmd_path.lines().next().unwrap_or(&cmd_path);
        fs::copy(cmd_path.trim_end(), &solc_path).unwrap();

        let mut child = if cfg!(target_os = "windows") {
            Command::new(&solc_path)
                .args(["/t", "3600"])
                .spawn()
                .unwrap()
        } else {
            Command::new(&solc_path).arg("infinity").spawn().unwrap()
        };

        // Install should not fail with "text file busy".
        install(&version).await.unwrap();

        child.kill().unwrap();
        let _: std::process::ExitStatus = child.wait().unwrap();
    }

    /// Ensures that an already installed binary that matches the expected checksum is reused
    /// instead of being replaced, even while it is currently being executed.
    ///
    /// Regression test for <https://github.com/foundry-rs/foundry/issues/4736>: replacing the
    /// installed binary on every install caused "text file busy" (`ETXTBSY`) errors when parallel
    /// processes installed and executed the same solc version.
    #[cfg(target_family = "unix")]
    #[serial_test::serial]
    #[test]
    fn install_reuses_existing_binary_while_running() {
        let version: Version = "0.8.19".parse().unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        fs::create_dir_all(solc_path.parent().unwrap()).unwrap();

        // Install a fake solc: a copy of `sleep`, so it can be executed while we "reinstall".
        let stdout = Command::new("which").arg("sleep").output().unwrap().stdout;
        let sleep_path = String::from_utf8(stdout).unwrap();
        fs::copy(sleep_path.trim_end(), &solc_path).unwrap();
        let binbytes = fs::read(&solc_path).unwrap();
        let expected_checksum = &sha2::Sha256::digest(&binbytes)[..];

        let mut child = Command::new(&solc_path).arg("30").spawn().unwrap();

        let _lock = try_lock_file(&lock_file_path(&version)).unwrap();
        let installed = do_install_and_retry(
            &version,
            b"different binary contents",
            "",
            expected_checksum,
        )
        .unwrap();

        assert_eq!(installed, solc_path);
        // The running binary matches the expected checksum and must not have been replaced.
        assert!(
            fs::read(&solc_path).unwrap() == binbytes,
            "the running binary was replaced"
        );

        child.kill().unwrap();
        let _: std::process::ExitStatus = child.wait().unwrap();
    }

    /// Ensures that an existing binary that does not match the expected checksum is replaced.
    #[serial_test::serial]
    #[test]
    fn install_replaces_corrupt_binary() {
        let version: Version = "0.8.21".parse().unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        fs::create_dir_all(solc_path.parent().unwrap()).unwrap();
        fs::write(&solc_path, b"corrupt binary contents").unwrap();

        let binbytes = b"expected binary contents";
        let expected_checksum = &sha2::Sha256::digest(binbytes)[..];

        let _lock = try_lock_file(&lock_file_path(&version)).unwrap();
        let installed = do_install_and_retry(&version, binbytes, "", expected_checksum).unwrap();

        assert_eq!(installed, solc_path);
        assert_eq!(fs::read(&solc_path).unwrap(), binbytes);
    }

    /// The lock file must never be removed, see [`try_lock_file`].
    #[serial_test::serial]
    #[test]
    fn lock_file_is_not_removed() {
        let version: Version = "0.8.13".parse().unwrap();
        setup_data_dir().unwrap();
        setup_version(version.to_string().as_str()).unwrap();

        let lock_path = lock_file_path(&version);
        drop(try_lock_file(&lock_path).unwrap());
        assert!(lock_path.exists());

        // Re-acquiring the lock must work with the file already present.
        drop(try_lock_file(&lock_path).unwrap());
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_install() {
        let versions = crate::releases::blocking_all_releases(platform::platform())
            .unwrap()
            .into_versions();
        let rand_version = versions.choose(&mut rand::rng()).unwrap();
        assert!(blocking_install(rand_version).is_ok());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_version() {
        let version = "0.8.10".parse().unwrap();
        install(&version).await.unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        let output = Command::new(solc_path).arg("--version").output().unwrap();
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .as_ref()
                .contains("0.8.10")
        );
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_latest() {
        blocking_install(&LATEST).unwrap();
        let solc_path = version_binary(LATEST.to_string().as_str());
        let output = Command::new(solc_path).arg("--version").output().unwrap();

        assert!(
            String::from_utf8_lossy(&output.stdout)
                .as_ref()
                .contains(&LATEST.to_string())
        );
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    fn blocking_test_version() {
        let version = "0.8.10".parse().unwrap();
        blocking_install(&version).unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        let output = Command::new(solc_path).arg("--version").output().unwrap();

        assert!(
            String::from_utf8_lossy(&output.stdout)
                .as_ref()
                .contains("0.8.10")
        );
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
        let version_output = String::from_utf8_lossy(&output.stdout);
        assert!(
            version_output.contains(&LATEST.to_string()),
            "{version_output}"
        );
    }

    // Ensures we can download the latest native solc for linux-aarch64.
    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    async fn can_download_linux_aarch64_latest() {
        let artifacts = all_releases(platform::Platform::LinuxAarch64)
            .await
            .unwrap();

        let artifact = artifacts.releases.get(&LATEST).unwrap();
        let download_url = artifact_url(
            platform::Platform::LinuxAarch64,
            &LATEST,
            artifact.to_string().as_str(),
        )
        .unwrap();

        let checksum = artifacts.get_checksum(&LATEST).unwrap();

        let resp = reqwest::get(download_url).await.unwrap();
        assert!(resp.status().is_success());
        let binbytes = resp.bytes().await.unwrap();
        ensure_checksum(&binbytes, &LATEST, &checksum).unwrap();
    }

    // Ensures we can download thirdparty linux-aarch64 solc binaries that do not have official
    // Solidity binary releases.
    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    async fn can_download_linux_aarch64_thirdparty() {
        let version: Version = "0.8.30".parse().unwrap();
        let artifacts = all_releases(platform::Platform::LinuxAarch64)
            .await
            .unwrap();

        let artifact = artifacts.releases.get(&version).unwrap();
        let download_url = artifact_url(
            platform::Platform::LinuxAarch64,
            &version,
            artifact.to_string().as_str(),
        )
        .unwrap();

        let checksum = artifacts.get_checksum(&version).unwrap();

        let resp = reqwest::get(download_url).await.unwrap();
        assert!(resp.status().is_success());
        let binbytes = resp.bytes().await.unwrap();
        ensure_checksum(&binbytes, &version, &checksum).unwrap();
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn can_install_windows_zip_release() {
        let version = "0.7.1".parse().unwrap();
        install(&version).await.unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        let output = Command::new(&solc_path).arg("--version").output().unwrap();

        assert!(
            String::from_utf8_lossy(&output.stdout)
                .as_ref()
                .contains("0.7.1")
        );
    }

    #[cfg(feature = "blocking")]
    #[serial_test::serial]
    #[test]
    #[ignore]
    fn blocking_test_0_8_31_pre() {
        let version = "0.8.31-pre.1".parse().unwrap();
        blocking_install(&version).unwrap();
        let solc_path = version_binary(version.to_string().as_str());
        let output = Command::new(solc_path).arg("--version").output().unwrap();

        assert!(
            String::from_utf8_lossy(&output.stdout)
                .as_ref()
                .contains(&version.to_string())
        );
    }
}
