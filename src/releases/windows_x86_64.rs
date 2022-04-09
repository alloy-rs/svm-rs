use reqwest::get;
use semver::Version;
use url::Url;

use super::{Releases, SOLC_RELEASES_URL};
use crate::{error::SolcVmError, platform::platform};

#[cfg(feature = "blocking")]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    Ok(
        reqwest::blocking::get(format!("{}/{}/list.json", SOLC_RELEASES_URL, platform(),))?
            .json::<Releases>()?,
    )
}

pub async fn all_releases() -> Result<Releases, SolcVmError> {
    Ok(
        get(format!("{}/{}/list.json", SOLC_RELEASES_URL, platform(),))
            .await?
            .json::<Releases>()
            .await?,
    )
}

pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    Ok(Url::parse(&format!(
        "{}/{}/{}",
        SOLC_RELEASES_URL,
        platform(),
        artifact
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_all_releases_windows() {
        assert!(all_releases().await.is_ok());
    }

    #[tokio::test]
    async fn releases_roundtrip() {
        let releases = all_releases().await.unwrap();
        let s = serde_json::to_string(&releases).unwrap();
        let de_releases: Releases = serde_json::from_str(&s).unwrap();
        assert_eq!(releases, de_releases);
    }
}
