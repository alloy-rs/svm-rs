use reqwest::get;
use semver::Version;
use url::Url;

use super::{Releases, SOLC_RELEASES_URL};
use crate::{error::SolcVmError, platform::platform};

/// The earliest version available for Platform::MacOsAmd64.
static OLD_VERSION_MIN: Lazy<Version> = Lazy::new(|| Version::new(0, 4, 0));

/// A blocking version to returns a list of all available releases that are supported for Platform::MacOsAmd64.
#[cfg(feature = "blocking")]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    Ok(
        reqwest::blocking::get(format!("{}/{}/list.json", SOLC_RELEASES_URL, platform()))?
            .json::<Releases>()?,
    )
}

/// Returns a list of all available releases that are supported for Platform::MacOsAmd64.
pub async fn all_releases() -> Result<Releases, SolcVmError> {
    Ok(
        get(format!("{}/{}/list.json", SOLC_RELEASES_URL, platform()))
            .await?
            .json::<Releases>()
            .await?,
    )
}

/// Constructs the URL to the solc binary with the given version and artifact for
/// Platform::MacOsAmd64.
pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    if version.lt(&OLD_VERSION_MIN) {
        Err(SolcVmError::UnsupportedVersion(
            version.to_string(),
            platform().to_string(),
        ))
    } else {
        Ok(Url::parse(&format!(
            "{}/{}/{}",
            SOLC_RELEASES_URL,
            platform(),
            artifact
        ))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_all_releases_macos_amd64() {
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
