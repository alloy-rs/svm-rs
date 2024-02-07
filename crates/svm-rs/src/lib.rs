#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(rustdoc::all)]
#![cfg_attr(
    not(any(test, feature = "cli", feature = "solc")),
    warn(unused_crate_dependencies)
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use semver::Version;
use sha2::Digest;
use std::{
    ffi::OsString,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::Duration,
};

// Use permission extensions on Unix.
#[cfg(target_family = "unix")]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

mod error;
pub use error::SvmError;

mod platform;
pub use platform::{platform, Platform};

mod releases;
pub use releases::{all_releases, Releases};

#[cfg(feature = "blocking")]
pub use releases::blocking_all_releases;

#[cfg(feature = "cli")]
#[doc(hidden)]
pub const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

/// The timeout to use for requests to the source
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Version beyond which solc binaries are not fully static, hence need to be patched for NixOS.
const NIXOS_MIN_PATCH_VERSION: Version = Version::new(0, 7, 6);

/// Returns the path to the default data directory.
///
/// Returns `~/.svm` if it exists, otherwise uses `$XDG_DATA_HOME/svm`.
pub fn data_dir() -> &'static Path {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        #[cfg(test)]
        {
            let dir = tempfile::tempdir().expect("could not create temp directory");
            dir.path().join(".svm")
        }
        #[cfg(not(test))]
        {
            resolve_data_dir()
        }
    })
}

fn resolve_data_dir() -> PathBuf {
    let home_dir = dirs::home_dir()
        .expect("could not detect user home directory")
        .join(".svm");

    let data_dir = dirs::data_dir().expect("could not detect user data directory");
    if !home_dir.exists() && data_dir.exists() {
        data_dir.join("svm")
    } else {
        home_dir
    }
}

/// Returns the path to the global version file.
pub fn global_version_path() -> &'static Path {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| data_dir().join(".global-version"))
}

/// Returns the path to a specific Solc version's directory.
///
/// Note that this is not the path to the actual Solc binary file;
/// use [`version_binary`] for that instead.
///
/// This is currently `data_dir() / {version}`.
pub fn version_path(version: &str) -> PathBuf {
    data_dir().join(version)
}

/// Derive path to a specific Solc version's binary file.
///
/// This is currently `data_dir() / {version} / solc-{version}`.
pub fn version_binary(version: &str) -> PathBuf {
    let data_dir = data_dir();
    let sep = std::path::MAIN_SEPARATOR_STR;
    let cap =
        data_dir.as_os_str().len() + sep.len() + version.len() + sep.len() + 5 + version.len();
    let mut binary = OsString::with_capacity(cap);
    binary.push(data_dir);
    debug_assert!(!data_dir.ends_with(sep));
    binary.push(sep);

    binary.push(version);
    binary.push(sep);

    binary.push("solc-");
    binary.push(version);
    PathBuf::from(binary)
}

/// Reads the currently set global version for Solc. Returns None if none has yet been set.
pub fn get_global_version() -> Result<Option<Version>, SvmError> {
    let v = fs::read_to_string(global_version_path())?;
    Ok(Version::parse(v.trim_end_matches('\n')).ok())
}

/// Sets the provided version as the global version for Solc.
pub fn set_global_version(version: &Version) -> Result<(), SvmError> {
    fs::write(global_version_path(), version.to_string()).map_err(Into::into)
}

/// Unset the global version. This should be done if all versions are removed.
pub fn unset_global_version() -> Result<(), SvmError> {
    fs::write(global_version_path(), "").map_err(Into::into)
}

/// Reads the list of Solc versions that have been installed in the machine.
/// The version list is sorted in ascending order.
pub fn installed_versions() -> Result<Vec<Version>, SvmError> {
    let mut versions = vec![];
    for v in fs::read_dir(data_dir())? {
        let v = v?;
        let path = v.path();
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if file_name == ".global-version" {
            continue;
        }
        versions.push(Version::parse(file_name)?);
    }
    versions.sort();
    Ok(versions)
}

/// Blocking version of [`all_versions`]
#[cfg(feature = "blocking")]
pub fn blocking_all_versions() -> Result<Vec<Version>, SvmError> {
    Ok(releases::blocking_all_releases(platform::platform())?.into_versions())
}

/// Fetches the list of all the available versions of Solc. The list is platform dependent, so
/// different versions can be found for macosx vs linux.
pub async fn all_versions() -> Result<Vec<Version>, SvmError> {
    Ok(releases::all_releases(platform::platform())
        .await?
        .into_versions())
}

/// Blocking version of [`install`]
#[cfg(feature = "blocking")]
pub fn blocking_install(version: &Version) -> Result<PathBuf, SvmError> {
    setup_data_dir()?;

    let artifacts = releases::blocking_all_releases(platform::platform())?;
    let artifact = artifacts
        .get_artifact(version)
        .ok_or(SvmError::UnknownVersion)?;
    let download_url =
        releases::artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

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

    let artifacts = releases::all_releases(platform::platform()).await?;
    let artifact = artifacts
        .releases
        .get(version)
        .ok_or(SvmError::UnknownVersion)?;
    let download_url =
        releases::artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

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

/// Removes the provided version of Solc from the machine.
pub fn remove_version(version: &Version) -> Result<(), SvmError> {
    fs::remove_dir_all(version_path(version.to_string().as_str())).map_err(Into::into)
}

/// Setup SVM home directory.
pub fn setup_data_dir() -> Result<(), SvmError> {
    // create $XDG_DATA_HOME or ~/.local/share/svm, or fallback to ~/.svm
    let data_dir = data_dir();

    // Create the directory, continuing if the directory came into existence after the check
    // for this if statement. This may happen if two copies of SVM run simultaneously (e.g CI).
    fs::create_dir_all(data_dir).or_else(|err| match err.kind() {
        ErrorKind::AlreadyExists => Ok(()),
        _ => Err(err),
    })?;

    // Check that the SVM directory is indeed a directory, and not e.g. a file.
    if !data_dir.is_dir() {
        return Err(SvmError::IoError(std::io::Error::new(
            ErrorKind::AlreadyExists,
            format!("{} is not a directory", data_dir.display()),
        )));
    }

    // Create `$SVM/.global-version`.
    let global_version = global_version_path();
    if !global_version.exists() {
        fs::File::create(global_version)?;
    }

    Ok(())
}

fn setup_version(version: &str) -> Result<(), SvmError> {
    let v = version_path(version);
    if !v.exists() {
        fs::create_dir_all(v)?;
    }
    Ok(())
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

/// Creates the file and locks it exclusively, this will block if the file is currently locked
fn try_lock_file(lock_path: PathBuf) -> Result<LockFile, SvmError> {
    use fs4::FileExt;
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
        let solc_path = version_binary(&self.version.to_string());

        let mut f = fs::File::create(&solc_path)?;
        #[cfg(target_family = "unix")]
        f.set_permissions(Permissions::from_mode(0o755))?;
        f.write_all(self.binbytes)?;

        if platform::is_nixos() && *self.version >= NIXOS_MIN_PATCH_VERSION {
            patch_for_nixos(&solc_path)?;
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        platform::Platform,
        releases::{all_releases, artifact_url},
    };
    use rand::seq::SliceRandom;
    use reqwest::Url;
    use std::process::Command;

    const LATEST: Version = Version::new(0, 8, 24);

    #[test]
    fn test_data_dir_resolution() {
        let home_dir = dirs::home_dir().unwrap().join(".svm");
        let data_dir = dirs::data_dir();
        let resolved_dir = resolve_data_dir();
        if home_dir.exists() || data_dir.is_none() {
            assert_eq!(resolved_dir, home_dir);
        } else {
            assert_eq!(resolved_dir, data_dir.unwrap().join("svm"));
        }
    }

    #[test]
    fn test_artifact_url() {
        let version = Version::new(0, 5, 0);
        let artifact = "solc-v0.5.0";
        assert_eq!(
            artifact_url(Platform::LinuxAarch64, &version, artifact).unwrap(),
            Url::parse(&format!(
                "https://github.com/nikitastupin/solc/raw/7687d6ce15553292adbb3e6c565eafea6e0caf85/linux/aarch64/{artifact}"
            ))
            .unwrap(),
        )
    }

    #[tokio::test]
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

    #[cfg(feature = "blocking")]
    #[test]
    fn blocking_test_install() {
        let versions = crate::releases::blocking_all_releases(platform::platform())
            .unwrap()
            .into_versions();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(blocking_install(rand_version).is_ok());
    }

    #[tokio::test]
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

    // ensures we can download the latest native solc for apple silicon
    #[tokio::test(flavor = "multi_thread")]
    async fn can_download_latest_native_apple_silicon() {
        let artifacts = all_releases(Platform::MacOsAarch64).await.unwrap();

        let artifact = artifacts.releases.get(&LATEST).unwrap();
        let download_url = artifact_url(
            Platform::MacOsAarch64,
            &LATEST,
            artifact.to_string().as_str(),
        )
        .unwrap();

        let expected_checksum = artifacts.get_checksum(&LATEST).unwrap();

        let resp = reqwest::get(download_url).await.unwrap();
        assert!(resp.status().is_success());
        let binbytes = resp.bytes().await.unwrap();
        ensure_checksum(&binbytes, &LATEST, &expected_checksum).unwrap();
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
