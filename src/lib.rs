use log::debug;
use once_cell::sync::Lazy;
use semver::Version;
use sha2::Digest;

use std::{
    ffi::OsString,
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    time::Duration,
};

/// Use permissions extensions on unix
#[cfg(target_family = "unix")]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

mod error;
pub use error::SolcVmError;

mod platform;
pub use platform::platform;

mod releases;
pub use releases::{all_releases, Releases};

static INSTALL_TIMEOUT: Duration = Duration::from_secs(10);
static LOCKFILE_CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// Declare path to Solc Version Manager's home directory, "~/.svm" on Unix-based machines.
pub static SVM_HOME: Lazy<PathBuf> = Lazy::new(|| {
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            let dir = tempfile::tempdir().expect("could not create temp directory");
            dir.path().join(".svm")
        } else {
            let mut user_home = home::home_dir().expect("could not detect user home directory");
            user_home.push(".svm");
            user_home
        }
    }
});

/// Derive path to a specific Solc version's binary.
pub fn version_path(version: &str) -> PathBuf {
    let mut version_path = SVM_HOME.to_path_buf();
    version_path.push(&version);
    version_path
}

/// Derive path to SVM's global version file.
pub fn global_version_path() -> PathBuf {
    let mut global_version_path = SVM_HOME.to_path_buf();
    global_version_path.push(".global-version");
    global_version_path
}

/// Reads the currently set global version for Solc. Returns None if none has yet been set.
pub fn current_version() -> Result<Option<Version>, SolcVmError> {
    let v = fs::read_to_string(global_version_path().as_path())?;
    Ok(Version::parse(v.trim_end_matches('\n').to_string().as_str()).ok())
}

/// Sets the provided version as the global version for Solc.
pub fn use_version(version: &Version) -> Result<(), SolcVmError> {
    let mut v = fs::File::create(global_version_path().as_path())?;
    v.write_all(version.to_string().as_bytes())?;
    Ok(())
}

/// Unset the global version. This should be done if all versions are removed.
pub fn unset_global_version() -> Result<(), SolcVmError> {
    let mut v = fs::File::create(global_version_path().as_path())?;
    v.write_all("".as_bytes())?;
    Ok(())
}

/// Reads the list of Solc versions that have been installed in the machine. The version list is
/// sorted in ascending order.
pub fn installed_versions() -> Result<Vec<Version>, SolcVmError> {
    let home_dir = SVM_HOME.to_path_buf();
    let mut versions = vec![];
    for v in fs::read_dir(&home_dir)? {
        let v = v?;
        if v.file_name() != OsString::from(".global-version".to_string()) {
            versions.push(Version::parse(
                v.path()
                    .file_name()
                    .ok_or(SolcVmError::UnknownVersion)?
                    .to_str()
                    .ok_or(SolcVmError::UnknownVersion)?
                    .to_string()
                    .as_str(),
            )?);
        }
    }
    versions.sort();
    Ok(versions)
}

/// Fetches the list of all the available versions of Solc. The list is platform dependent, so
/// different versions can be found for macosx vs linux.
pub async fn all_versions() -> Result<Vec<Version>, SolcVmError> {
    let mut releases = releases::all_releases(platform::platform())
        .await?
        .releases
        .keys()
        .cloned()
        .collect::<Vec<Version>>();
    releases.sort();
    Ok(releases)
}

/// Installs the provided version of Solc in the machine.
pub async fn install(version: &Version) -> Result<(), SolcVmError> {
    setup_home()?;

    let artifacts = releases::all_releases(platform::platform()).await?;
    let artifact = artifacts
        .releases
        .get(version)
        .ok_or(SolcVmError::UnknownVersion)?;
    let download_url =
        releases::artifact_url(platform::platform(), version, artifact.to_string().as_str())?;

    let res = reqwest::get(download_url).await?;
    let binbytes = res.bytes().await?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&binbytes);
    let cs = &hasher.finalize()[..];
    let checksum = artifacts
        .get_checksum(version)
        .unwrap_or_else(|| panic!("checksum not available: {:?}", version.to_string()));

    // checksum does not match
    if cs != checksum {
        return Err(SolcVmError::ChecksumMismatch(version.to_string()));
    }

    let lock_path = SVM_HOME.join(&format!(".lock-solc-{}", version));
    let version_path = version_path(version.to_string().as_str());
    let solc_path = version_path.join(&format!("solc-{}", version));

    // wait until lock file is released, possibly by another parallel thread trying to install the
    // same version of solc.
    tokio::time::timeout(INSTALL_TIMEOUT, wait_for_lock(&lock_path))
        .await
        .map_err(|_| SolcVmError::Timeout(version.to_string(), INSTALL_TIMEOUT.as_secs()))?;

    let mut dest = {
        setup_version(version.to_string().as_str())?;

        let f = fs::File::create(&solc_path)?;

        #[cfg(target_family = "unix")]
        f.set_permissions(Permissions::from_mode(0o777))?;

        f
    };

    // create lock file before copying contents.
    fs::File::create(&lock_path)?;

    // copy contents over
    let mut content = Cursor::new(binbytes);
    std::io::copy(&mut content, &mut dest)?;

    // delete lock file
    fs::remove_file(&lock_path)?;

    Ok(())
}

/// Removes the provided version of Solc from the machine.
pub fn remove_version(version: &Version) -> Result<(), SolcVmError> {
    fs::remove_dir_all(version_path(version.to_string().as_str()))?;
    Ok(())
}

/// Setup SVM home directory.
pub fn setup_home() -> Result<PathBuf, SolcVmError> {
    // create ~/.svm
    let home_dir = SVM_HOME.to_path_buf();
    if !home_dir.as_path().exists() {
        fs::create_dir_all(home_dir.clone())?;
    }
    // create ~/.svm/.global-version
    let mut global_version = SVM_HOME.to_path_buf();
    global_version.push(".global-version");
    if !global_version.as_path().exists() {
        fs::File::create(global_version.as_path())?;
    }
    Ok(home_dir)
}

async fn wait_for_lock(lock_path: &Path) {
    let mut interval = tokio::time::interval(LOCKFILE_CHECK_INTERVAL);
    while lock_path.exists() {
        interval.tick().await;
        debug!("waiting for lock file to be released");
    }
}

fn setup_version(version: &str) -> Result<(), SolcVmError> {
    let v = version_path(version);
    if !v.exists() {
        fs::create_dir_all(v.as_path())?
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        platform::Platform,
        releases::{all_releases, artifact_url},
    };
    use rand::seq::SliceRandom;
    use reqwest::Url;
    use std::process::Command;
    use std::process::Stdio;

    use super::*;

    #[tokio::test]
    async fn test_artifact_url() {
        let version = Version::new(0, 5, 0);
        let artifact = "solc-v0.5.0";
        assert_eq!(
            artifact_url(Platform::LinuxAarch64, &version, artifact).unwrap(),
            Url::parse(&format!(
                "https://github.com/nikitastupin/solc/raw/3890b86a62fe6b8efd2f643f4adcd854f478b623/linux/aarch64/{}",
                artifact
            ))
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn test_install() {
        let versions = all_releases(platform::platform())
            .await
            .unwrap()
            .releases
            .into_keys()
            .collect::<Vec<Version>>();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(install(rand_version).await.is_ok());
    }

    #[tokio::test]
    async fn test_version() {
        let version = "0.8.10".parse().unwrap();
        install(&version).await.unwrap();
        let solc_path =
            version_path(version.to_string().as_str()).join(&format!("solc-{}", version));
        let output = Command::new(&solc_path)
            .arg("--version")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        assert!(String::from_utf8_lossy(&output.stdout)
            .as_ref()
            .contains("0.8.10"));
    }
}
