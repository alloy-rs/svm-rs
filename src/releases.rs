use once_cell::sync::Lazy;
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
const OLD_SOLC_RELEASES_DOWNLOAD_PREFIX: &str =
    "https://raw.githubusercontent.com/crytic/solc/master/linux/amd64";

static OLD_VERSION_MAX: Lazy<Version> = Lazy::new(|| Version::new(0, 4, 9));

static OLD_VERSION_MIN: Lazy<Version> = Lazy::new(|| Version::new(0, 4, 0));

static OLD_SOLC_RELEASES: Lazy<Releases> = Lazy::new(|| {
    serde_json::from_reader(
        std::fs::File::open("./list/linux-arm64-old.json")
            .expect("could not open list linux-arm64-old.json"),
    )
    .expect("could not parse list linux-arm64-old.json")
});

static LINUX_AARCH64_URL_PREFIX: &str =
    "https://github.com/nikitastupin/solc/raw/main/linux/aarch64";

static LINUX_AARCH64_RELEASES: Lazy<Releases> = Lazy::new(|| {
    serde_json::from_reader(
        std::fs::File::open("./list/linux-aarch64.json")
            .expect("could not open list linux-aarch64.json"),
    )
    .expect("could not parse list linux-aarch64.json")
});

/// Defines the struct that the JSON-formatted release list can be deserialized into.
///
/// {
///     "builds": [
///         {
///             "version": "0.8.7",
///             "sha256": "0x0xcc5c663d1fe17d4eb4aca09253787ac86b8785235fca71d9200569e662677990"
///         }
///     ]
///     "releases": {
///         "0.8.7": "solc-macosx-amd64-v0.8.7+commit.e28d00a7",
///         "0.8.6": "solc-macosx-amd64-v0.8.6+commit.11564f7e",
///         ...
///     }
/// }
///
/// Both the key and value are deserialized into semver::Version.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Releases {
    pub builds: Vec<BuildInfo>,
    #[serde(deserialize_with = "de_releases")]
    pub releases: HashMap<Version, String>,
}

impl Releases {
    /// Get the checksum of a solc version's binary if it exists.
    pub fn get_checksum(&self, v: &Version) -> Option<Vec<u8>> {
        for build in self.builds.iter() {
            if build.version.eq(v) {
                return Some(build.sha256.clone());
            }
        }
        None
    }
}

/// Build info contains the SHA256 checksum of a solc binary.
#[derive(Clone, Debug, Deserialize)]
pub struct BuildInfo {
    #[serde(deserialize_with = "version_from_string")]
    pub version: Version,
    #[serde(deserialize_with = "from_hex_string")]
    pub sha256: Vec<u8>,
}

/// Helper to parse hex string to a vector.
fn from_hex_string<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_hex = String::deserialize(deserializer)?;
    let str_hex = str_hex.trim_start_matches("0x");
    hex::decode(str_hex).map_err(|err| de::Error::custom(err.to_string()))
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
    if platform == Platform::LinuxAarch64 {
        return Ok(LINUX_AARCH64_RELEASES.clone());
    }

    let releases = get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        platform.to_string()
    ))
    .await?
    .json::<Releases>()
    .await?;

    if platform == Platform::LinuxAmd64 {
        let mut all_releases = OLD_SOLC_RELEASES.clone();
        all_releases.builds.extend(releases.builds);
        all_releases.releases.extend(releases.releases);
        return Ok(all_releases);
    }

    Ok(releases)
}

/// Construct the URL to the Solc binary for the specified release version and target platform.
pub fn artifact_url(
    platform: Platform,
    version: &Version,
    artifact: &str,
) -> Result<Url, SolcVmError> {
    if platform == Platform::LinuxAmd64
        && version.le(&OLD_VERSION_MAX)
        && version.ge(&OLD_VERSION_MIN)
    {
        return Ok(Url::parse(&format!(
            "{}/{}",
            OLD_SOLC_RELEASES_DOWNLOAD_PREFIX, artifact
        ))?);
    }

    if platform == Platform::LinuxAarch64 {
        if LINUX_AARCH64_RELEASES.releases.contains_key(version) {
            return Ok(Url::parse(&format!(
                "{}/{}",
                LINUX_AARCH64_URL_PREFIX, artifact
            ))?);
        } else {
            return Err(SolcVmError::UnsupportedVersion(
                version.to_string(),
                platform.to_string(),
            ));
        }
    }

    if platform == Platform::MacOsAmd64 && version.lt(&OLD_VERSION_MIN) {
        return Err(SolcVmError::UnsupportedVersion(
            version.to_string(),
            platform.to_string(),
        ));
    }

    Ok(Url::parse(&format!(
        "{}/{}/{}",
        SOLC_RELEASES_URL,
        platform.to_string(),
        artifact
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_old_releases_deser() {
        assert_eq!(OLD_SOLC_RELEASES.releases.len(), 10);
        assert_eq!(OLD_SOLC_RELEASES.builds.len(), 10);
    }

    #[test]
    fn test_linux_aarch64() {
        assert_eq!(LINUX_AARCH64_RELEASES.releases.len(), 43);
        assert_eq!(LINUX_AARCH64_RELEASES.builds.len(), 43);
    }

    #[tokio::test]
    async fn test_all_releases_macos() {
        assert!(all_releases(Platform::MacOsAmd64).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_linux() {
        assert!(all_releases(Platform::LinuxAmd64).await.is_ok());
    }
}
