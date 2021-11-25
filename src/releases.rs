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
    serde_json::from_str(
        r#"{
            "builds": [
                {
                    "version": "0.4.9",
                    "sha256": "0x71da154585e0c9048445b39b3662b421d20814cc68482b6b072aae2e541a4c74"
                },
                {
                    "version": "0.4.8",
                    "sha256": "0xee76039f933938cb5c14bf3fc4754776aa3e5c4c88420413da4c0c13731b8ffe"
                },
                {
                    "version": "0.4.7",
                    "sha256": "0xe1affb6e13dee7b14039f8eb1a52343f5fdc56169023e0f7fc339dfc25ad2b3d"
                },
                {
                    "version": "0.4.6",
                    "sha256": "0x0525d7b95549db6c913edb3c1b0c26d2db81e64b03f8352261df1b2ad696a65e"
                },
                {
                    "version": "0.4.5",
                    "sha256": "0x6f46ab7747d7de1b75907e539e6e19be201680e64ce99b583c6356e4e7897406"
                },
                {
                    "version": "0.4.4",
                    "sha256": "0x25d148e9c1052631a930bfbe8e4e3d9dae8de7659f8d3ea659a3ef139cd5e2c9"
                },
                {
                    "version": "0.4.3",
                    "sha256": "0x1dc7ef0b4aab472299e77b39c7465cd5ed4609a759b52ce1a93f2d54395da73a"
                },
                {
                    "version": "0.4.2",
                    "sha256": "0x891d0b2d3a636ff40924802a6f5beb1ecbc42d5d0d0bfecbbb148b561c861fb9"
                },
                {
                    "version": "0.4.1",
                    "sha256": "0xa0c06d0c6a14c66ddeca1f065461fb0024de89421c1809a1b103b55c94e30860"
                },
                {
                    "version": "0.4.0",
                    "sha256": "0xe26d188284763684f3cf6d4900b72f7e45a050dd2b2707320273529d033cfd47"
                }
            ],
            "releases": {
                "0.4.9": "solc-v0.4.9",
                "0.4.8": "solc-v0.4.8",
                "0.4.7": "solc-v0.4.7",
                "0.4.6":"solc-v0.4.6",
                "0.4.5": "solc-v0.4.5",
                "0.4.4":"solc-v0.4.4",
                "0.4.3":"solc-v0.4.3",
                "0.4.2":"solc-v0.4.2",
                "0.4.1": "solc-v0.4.1",
                "0.4.0": "solc-v0.4.0"
            }
        }"#,
    )
    .expect("could not parse old solc list.json")
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
        assert_eq!(OLD_SOLC_RELEASES.releases.len(), 11);
        assert_eq!(OLD_SOLC_RELEASES.builds.len(), 11);
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
