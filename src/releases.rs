use reqwest::get;
use semver::Version;
use serde::{
    de::{self, Deserializer},
    Deserialize,
};
use std::collections::HashMap;
use url::Url;

use crate::{error::SolcVmError, platform::Platform};

const SOLC_RELEASES_URL: &str = "https://binaries.soliditylang.org";
const OLD_SOLC_RELEASES: &str =
    "https://raw.githubusercontent.com/crytic/solc/list-json/linux/amd64";

/// Defines the struct that the JSON-formatted release list can be deserialized into.
///
/// {
///     "releases": {
///         "0.8.7": "solc-macosx-amd64-v0.8.7+commit.e28d00a7",
///         "0.8.6": "solc-macosx-amd64-v0.8.6+commit.11564f7e",
///         ...
///     }
/// }
///
/// Both the key and value are deserialized into semver::Version.
#[derive(Debug, Deserialize)]
pub struct Releases {
    #[serde(deserialize_with = "de_releases")]
    pub releases: HashMap<Version, String>,
}

/// Helper to parse string to semver::Version.
fn version_from_string<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    let str_version = String::deserialize(deserializer)?;
    Version::parse(&str_version).map_err(|err| de::Error::custom(err.to_string()))
}

/// Custom deserializer that deserializes a map of <String, String> to <Version, Version>.
fn de_releases<'de, D>(deserializer: D) -> Result<HashMap<Version, String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(PartialEq, Eq, Hash, Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "version_from_string")] Version);

    let v = HashMap::<Wrapper, String>::deserialize(deserializer)?;
    Ok(v.into_iter().map(|(Wrapper(k), v)| (k, v)).collect())
}

/// Fetch all releases available for the provided platform.
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

/// Construct the URL to the Solc binary for the specified release version and target platform.
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
