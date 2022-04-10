use once_cell::sync::Lazy;
use reqwest::get;
use semver::Version;
use url::Url;

use super::{Releases, SOLC_RELEASES_URL};
use crate::{error::SolcVmError, platform::platform};

/// Prefix to the URL for solc binaries for versions marked old, specifically 0.4.0 <= v <= 0.4.9.
const OLD_RELEASES_PREFIX: &str =
    "https://raw.githubusercontent.com/crytic/solc/master/linux/amd64";

/// Earliest version available for Platform::LinuxAmd64.
static OLD_VERSION_MIN: Lazy<Version> = Lazy::new(|| Version::new(0, 4, 0));

/// Latest version that has been marked old, available for Platform::LinuxAmd64.
static OLD_VERSION_MAX: Lazy<Version> = Lazy::new(|| Version::new(0, 4, 9));

/// Returns a list of all the releases marked old, specifically from 0.4.0 to 0.4.9 and supported
/// by Platform::LinuxAmd64.
static OLD_RELEASES: Lazy<Releases> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../list/linux-arm64-old.json"))
        .expect("could not parse list linux-arm64-old.json")
});

/// A blocking version to returns a list of all available releases that are supported for Platform::LinuxAmd64.
#[cfg(feature = "blocking")]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    let releases =
        reqwest::blocking::get(format!("{}/{}/list.json", SOLC_RELEASES_URL, platform()))?
            .json::<Releases>()?;
    let mut all_releases = OLD_RELEASES.clone();
    all_releases.builds.extend(releases.builds);
    all_releases.releases.extend(releases.releases);
    Ok(all_releases)
}

/// Returns a list of all available releases that are supported for Platform::LinuxAmd64.
pub async fn all_releases() -> Result<Releases, SolcVmError> {
    let releases = get(format!("{}/{}/list.json", SOLC_RELEASES_URL, platform()))
        .await?
        .json::<Releases>()
        .await?;
    let mut all_releases = OLD_RELEASES.clone();
    all_releases.builds.extend(releases.builds);
    all_releases.releases.extend(releases.releases);
    Ok(all_releases)
}

/// Constructs the URL to the solc binary with the given version and artifact for
/// Platform::LinuxAmd64.
pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    if version.le(&OLD_VERSION_MAX) && version.ge(&OLD_VERSION_MIN) {
        return Ok(Url::parse(&format!(
            "{}/{}",
            OLD_RELEASES_PREFIX, artifact,
        ))?);
    }
    Ok(Url::parse(&format!(
        "{}/{}/{}",
        SOLC_RELEASES_URL,
        platform(),
        artifact,
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_old_releases_deser() {
        assert_eq!(OLD_RELEASES.releases.len(), 10);
        assert_eq!(OLD_RELEASES.builds.len(), 10);
    }

    #[tokio::test]
    async fn test_all_releases_linux_amd64() {
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
