use reqwest::get;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

use crate::{error::SolcVmError, platform::Platform};

const SOLC_RELEASES_URL: &str = "https://binaries.soliditylang.org";
const OLD_SOLC_RELEASES: &str =
    "https://raw.githubusercontent.com/crytic/solc/list-json/linux/amd64";

#[derive(Debug, Deserialize)]
pub struct Releases {
    pub releases: HashMap<String, String>,
}

pub async fn all_releases(platform: Platform) -> Result<Releases, SolcVmError> {
    let releases = get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        platform.to_string()
    ))
    .await?
    .json::<Releases>()
    .await?;

    if platform == Platform::LinuxAmd64 {
        let mut all_releases = get(format!("{}/list.json", OLD_SOLC_RELEASES))
            .await?
            .json::<Releases>()
            .await?;

        all_releases.releases.extend(releases.releases);
        return Ok(all_releases);
    }

    Ok(releases)
}

pub fn artifact_url(platform: Platform, version: String) -> Result<Url, SolcVmError> {
    if platform == Platform::MacOsAmd64 && version.as_str() < "0.4.10" {
        return Err(SolcVmError::UnsupportedVersion(
            version,
            platform.to_string(),
        ));
    }

    if version.as_str() < "0.4.10" {
        return Ok(Url::parse(&format!("{}/{}", OLD_SOLC_RELEASES, version))?);
    }

    Ok(Url::parse(&format!(
        "{}/{}/{}",
        SOLC_RELEASES_URL,
        platform.to_string(),
        version
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_all_releases_macos() {
        assert!(all_releases(Platform::MacOsAmd64).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_linux() {
        assert!(all_releases(Platform::LinuxAmd64).await.is_ok());
    }
}
