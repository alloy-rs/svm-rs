use once_cell::sync::Lazy;

use std::{
    fs::{self, Permissions},
    io::{Cursor, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

mod error;
mod platform;
mod releases;
use error::SolcVmError;

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

pub fn current_version() -> Result<String, SolcVmError> {
    let v = fs::read_to_string(global_version_path().as_path())?;
    Ok(v.trim_end_matches('\n').to_string())
}

pub fn installed_versions() -> Result<Vec<String>, SolcVmError> {
    let home_dir = SVM_HOME.to_path_buf();
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
    let mut v = fs::File::create(global_version_path().as_path())?;
    v.write_all(version.as_bytes())?;
    Ok(())
}

pub async fn install(version: &str) -> Result<(), SolcVmError> {
    setup_home()?;

    let artifacts = releases::all_releases(platform::platform()).await?;
    let artifact = artifacts
        .releases
        .get(version)
        .ok_or(SolcVmError::UnknownVersion)?;
    let download_url = releases::artifact_url(platform::platform(), artifact.to_string())?;

    let res = reqwest::get(download_url).await?;

    let mut dest = {
        setup_version(version)?;
        let fname = version_path(version).join(&format!("solc-{}", version));
        let f = fs::File::create(fname)?;
        f.set_permissions(Permissions::from_mode(0o777))?;
        f
    };

    let mut content = Cursor::new(res.bytes().await?);
    std::io::copy(&mut content, &mut dest)?;

    Ok(())
}

fn setup_home() -> Result<PathBuf, SolcVmError> {
    let home_dir = SVM_HOME.to_path_buf();
    if !home_dir.as_path().exists() {
        fs::create_dir_all(home_dir.clone())?;
    }
    Ok(home_dir)
}

fn setup_version(version: &str) -> Result<(), SolcVmError> {
    let v = version_path(version);
    if !v.exists() {
        fs::create_dir_all(v.as_path())?
    }
    Ok(())
}

pub fn version_path(version: &str) -> PathBuf {
    let mut version_path = SVM_HOME.to_path_buf();
    version_path.push(&version);
    version_path
}

pub fn global_version_path() -> PathBuf {
    let mut global_version_path = SVM_HOME.to_path_buf();
    global_version_path.push(".global-version");
    global_version_path
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
