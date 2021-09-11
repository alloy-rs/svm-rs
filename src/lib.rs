#[macro_use]
extern crate lazy_static;

use std::{
    fs::{self, Permissions},
    io::{Cursor, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

mod error;
mod platform;
mod releases;
use error::SolcVmError;

lazy_static! {
    static ref SOLC_VM_HOME: PathBuf = {
        let mut user_home = home::home_dir().expect("could not detect home directory");
        user_home.push(".solc-vm");
        user_home
    };
    static ref SOLC_GLOBAL_VERSION: PathBuf = {
        let mut global_version = SOLC_VM_HOME.to_path_buf();
        global_version.push(".global-version");
        global_version
    };
}

pub fn home() -> PathBuf {
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            let dir = tempfile::tempdir().expect("could not create temp dir");
            dir.path().join(".solc-vm")
        } else {
            SOLC_VM_HOME.to_path_buf()
        }
    }
}

pub fn current_version() -> Result<String, SolcVmError> {
    let v = fs::read_to_string(SOLC_GLOBAL_VERSION.as_path())?;
    Ok(v.trim_end_matches('\n').to_string())
}

pub fn installed_versions() -> Result<Vec<String>, SolcVmError> {
    let home_dir = home();
    let mut versions = vec![];
    for version in fs::read_dir(&home_dir)? {
        let version = version?;
        versions.push(
            version
                .path()
                .file_name()
                .ok_or(SolcVmError::UnknownVersion)?
                .to_str()
                .ok_or(SolcVmError::UnknownVersion)?
                .to_string(),
        );
    }

    versions.sort();
    Ok(versions)
}

pub async fn all_versions() -> Result<Vec<String>, SolcVmError> {
    Ok(releases::all_releases(platform::platform())
        .await?
        .releases
        .keys()
        .cloned()
        .collect::<Vec<String>>())
}

pub fn use_version(version: &str) -> Result<(), SolcVmError> {
    let mut v = fs::File::create(SOLC_GLOBAL_VERSION.as_path())?;
    v.write_all(version.as_bytes())?;
    Ok(())
}

pub async fn install(version: &str) -> Result<(), SolcVmError> {
    let home_dir = setup_home()?;

    let artifacts = releases::all_releases(platform::platform()).await?;
    let artifact = artifacts
        .releases
        .get(version)
        .ok_or(SolcVmError::UnknownVersion)?;
    let download_url = releases::artifact_url(platform::platform(), artifact.to_string())?;

    let res = reqwest::get(download_url).await?;

    let mut dest = {
        setup(&home_dir, version)?;
        let fname = version_path(&home_dir, version).join(&format!("solc-{}", version));
        let f = fs::File::create(fname)?;
        f.set_permissions(Permissions::from_mode(0o777))?;
        f
    };

    let mut content = Cursor::new(res.bytes().await?);
    std::io::copy(&mut content, &mut dest)?;

    Ok(())
}

fn setup_home() -> Result<PathBuf, SolcVmError> {
    let home_dir = home();
    if !home_dir.as_path().exists() {
        fs::create_dir_all(home_dir.clone())?;
    }
    Ok(home_dir)
}

fn setup(home_dir: &Path, version: &str) -> Result<(), SolcVmError> {
    let v = version_path(home_dir, version);
    if !v.exists() {
        fs::create_dir_all(v.as_path())?
    }
    Ok(())
}

pub fn version_path(home_dir: &Path, version: &str) -> PathBuf {
    let mut version_path = home_dir.to_path_buf();
    version_path.push(&version);
    version_path
}

#[cfg(test)]
mod tests {
    use crate::releases::all_releases;
    use rand::seq::SliceRandom;

    use super::*;

    #[tokio::test]
    async fn test_install() {
        let versions = all_releases(platform::platform())
            .await
            .unwrap()
            .releases
            .into_keys()
            .collect::<Vec<String>>();
        let rand_version = versions.choose(&mut rand::thread_rng()).unwrap();
        assert!(install(&rand_version).await.is_ok());
    }
}
