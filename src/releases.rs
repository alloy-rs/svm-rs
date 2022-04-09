use once_cell::sync::Lazy;
use reqwest::get;
use semver::Version;
use serde::{
    de::{self, Deserializer},
    Deserialize, Serialize,
};
use std::collections::BTreeMap;
use url::Url;

use crate::error::SolcVmError;

const SOLC_RELEASES_URL: &str = "https://binaries.soliditylang.org";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const OLD_SOLC_RELEASES_DOWNLOAD_PREFIX: &str =
    "https://raw.githubusercontent.com/crytic/solc/master/linux/amd64";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
static OLD_VERSION_MAX: Lazy<Version> = Lazy::new(|| Version::new(0, 4, 9));

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
static OLD_VERSION_MIN: Lazy<Version> = Lazy::new(|| Version::new(0, 4, 0));

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
static OLD_SOLC_RELEASES: Lazy<Releases> = Lazy::new(|| {
    serde_json::from_str(include_str!("../list/linux-arm64-old.json"))
        .expect("could not parse list linux-arm64-old.json")
});

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
static LINUX_AARCH64_URL_PREFIX: &str =
    "https://github.com/nikitastupin/solc/raw/c41a97910d37ee3b2c32ae2a3f8b6b8667e3fb39/linux/aarch64";

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
static LINUX_AARCH64_RELEASES: Lazy<Releases> = Lazy::new(|| {
    serde_json::from_str(include_str!("../list/linux-aarch64.json"))
        .expect("could not parse list linux-aarch64.json")
});

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
static MACOS_AARCH64_NATIVE: Lazy<Version> = Lazy::new(|| Version::new(0, 8, 5));

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
static MACOS_AARCH64_URL_PREFIX: &str =
    "https://github.com/roynalnaruto/solc-builds/raw/465839dcbb23fd4e60c16e8cae32513cd5627ca0/macosx/aarch64";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
static MACOS_AARCH64_RELEASES_URL: &str =
    "https://github.com/roynalnaruto/solc-builds/raw/465839dcbb23fd4e60c16e8cae32513cd5627ca0/macosx/aarch64/list.json";

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
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Releases {
    pub builds: Vec<BuildInfo>,
    pub releases: BTreeMap<Version, String>,
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

    /// Returns the artifact of the version if any
    pub fn get_artifact(&self, version: &Version) -> Option<&String> {
        self.releases.get(version)
    }

    /// Returns a sorted list of all versions
    pub fn into_versions(self) -> Vec<Version> {
        let mut versions = self.releases.into_keys().collect::<Vec<_>>();
        versions.sort_unstable();
        versions
    }
}

/// Build info contains the SHA256 checksum of a solc binary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: Version,
    #[serde(with = "hex_string")]
    pub sha256: Vec<u8>,
}

/// Helper serde module to serialize and deserialize bytes as hex.
mod hex_string {
    use super::*;
    use serde::Serializer;
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str_hex = String::deserialize(deserializer)?;
        let str_hex = str_hex.trim_start_matches("0x");
        hex::decode(str_hex).map_err(|err| de::Error::custom(err.to_string()))
    }

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: AsRef<[u8]>,
    {
        let value = hex::encode(value);
        serializer.serialize_str(&value)
    }
}

#[cfg(all(feature = "blocking", target_os = "linux", target_arch = "x86_64"))]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    let releases = reqwest::blocking::get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        crate::platform()
    ))?
    .json::<Releases>()?;
    let mut all_releases = OLD_SOLC_RELEASES.clone();
    all_releases.builds.extend(releases.builds);
    all_releases.releases.extend(releases.releases);
    Ok(all_releases)
}

#[cfg(all(feature = "blocking", target_os = "linux", target_arch = "aarch64"))]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    Ok(LINUX_AARCH64_RELEASES.clone())
}

#[cfg(all(feature = "blocking", target_os = "macos", target_arch = "x86_64"))]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    Ok(reqwest::blocking::get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        crate::platform()
    ))?
    .json::<Releases>()?)
}

// The supported versions for both macos-amd64 and macos-aarch64 are the same.
//
// 1. For version >= 0.8.5 we fetch native releases from
// https://github.com/roynalnaruto/solc-builds
//
// 2. For version <= 0.8.4 we fetch releases from https://binaries.soliditylang.org and
// require Rosetta support.
#[cfg(all(feature = "blocking", target_os = "macos", target_arch = "aarch64"))]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    let mut native = reqwest::blocking::get(MACOS_AARCH64_RELEASES_URL)?.json::<Releases>()?;
    let mut releases = reqwest::blocking::get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        crate::platform::Platform::MacOsAmd64,
    ))?
    .json::<Releases>()?;
    releases.builds = releases
        .builds
        .iter()
        .filter(|b| b.version.lt(&MACOS_AARCH64_NATIVE))
        .cloned()
        .collect();
    releases.builds.extend_from_slice(&native.builds);
    releases.releases.append(&mut native.releases);
    Ok(releases)
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    Ok(reqwest::blocking::get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        crate::platform()
    ))?
    .json::<Releases>()?)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub async fn all_releases() -> Result<Releases, SolcVmError> {
    let releases = get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        crate::platform()
    ))
    .await?
    .json::<Releases>()
    .await?;
    let mut all_releases = OLD_SOLC_RELEASES.clone();
    all_releases.builds.extend(releases.builds);
    all_releases.releases.extend(releases.releases);
    Ok(all_releases)
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub async fn all_releases() -> Result<Releases, SolcVmError> {
    Ok(LINUX_AARCH64_RELEASES.clone())
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub async fn all_releases() -> Result<Releases, SolcVmError> {
    Ok(get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        crate::platform()
    ))
    .await?
    .json::<Releases>()
    .await?)
}

// The supported versions for both macos-amd64 and macos-aarch64 are the same.
//
// 1. For version >= 0.8.5 we fetch native releases from
// https://github.com/roynalnaruto/solc-builds
//
// 2. For version <= 0.8.4 we fetch releases from https://binaries.soliditylang.org and
// require Rosetta support.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub async fn all_releases() -> Result<Releases, SolcVmError> {
    let mut native = get(MACOS_AARCH64_RELEASES_URL)
        .await?
        .json::<Releases>()
        .await?;
    let mut releases = get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        crate::platform::Platform::MacOsAmd64
    ))
    .await?
    .json::<Releases>()
    .await?;
    releases.builds = releases
        .builds
        .iter()
        .filter(|b| b.version.lt(&MACOS_AARCH64_NATIVE))
        .cloned()
        .collect();
    releases.builds.extend_from_slice(&native.builds);
    releases.releases.append(&mut native.releases);
    Ok(releases)
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub async fn all_releases() -> Result<Releases, SolcVmError> {
    Ok(get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        crate::platform()
    ))
    .await?
    .json::<Releases>()
    .await?)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    if version.le(&OLD_VERSION_MAX) && version.ge(&OLD_VERSION_MIN) {
        return Ok(Url::parse(&format!(
            "{}/{}",
            OLD_SOLC_RELEASES_DOWNLOAD_PREFIX, artifact
        ))?);
    }
    Ok(Url::parse(&format!(
        "{}/{}/{}",
        SOLC_RELEASES_URL,
        crate::platform(),
        artifact
    ))?)
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    if LINUX_AARCH64_RELEASES.releases.contains_key(version) {
        Ok(Url::parse(&format!(
            "{}/{}",
            LINUX_AARCH64_URL_PREFIX, artifact
        ))?)
    } else {
        Err(SolcVmError::UnsupportedVersion(
            version.to_string(),
            crate::platform().to_string(),
        ))
    }
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    if version.lt(&OLD_VERSION_MIN) {
        Err(SolcVmError::UnsupportedVersion(
            version.to_string(),
            crate::platform().to_string(),
        ))
    } else {
        Ok(Url::parse(&format!(
            "{}/{}/{}",
            SOLC_RELEASES_URL,
            crate::platform(),
            artifact
        ))?)
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    if version.ge(&MACOS_AARCH64_NATIVE) {
        Ok(Url::parse(&format!(
            "{}/{}",
            MACOS_AARCH64_URL_PREFIX, artifact
        ))?)
    } else {
        Ok(Url::parse(&format!(
            "{}/{}/{}",
            SOLC_RELEASES_URL,
            crate::platform::Platform::MacOsAmd64,
            artifact,
        ))?)
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    Ok(Url::parse(&format!(
        "{}/{}/{}",
        SOLC_RELEASES_URL,
        crate::platform(),
        artifact
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn test_old_releases_deser() {
        assert_eq!(OLD_SOLC_RELEASES.releases.len(), 10);
        assert_eq!(OLD_SOLC_RELEASES.builds.len(), 10);
    }

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    fn test_linux_aarch64() {
        assert_eq!(LINUX_AARCH64_RELEASES.releases.len(), 45);
        assert_eq!(LINUX_AARCH64_RELEASES.builds.len(), 45);
    }

    #[tokio::test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    async fn test_macos_aarch64() {
        let releases = all_releases()
            .await
            .expect("could not fetch releases for macos-aarch64");
        let rosetta = Version::new(0, 8, 4);
        let native = MACOS_AARCH64_NATIVE.clone();
        let url1 = artifact_url(&rosetta, releases.get_artifact(&rosetta).unwrap())
            .expect("could not fetch artifact URL");
        let url2 = artifact_url(&native, releases.get_artifact(&native).unwrap())
            .expect("could not fetch artifact URL");
        assert!(url1.to_string().contains(SOLC_RELEASES_URL));
        assert!(url2.to_string().contains(MACOS_AARCH64_URL_PREFIX));
    }

    #[tokio::test]
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    async fn test_all_releases_macos_amd64() {
        assert!(all_releases().await.is_ok());
    }

    #[tokio::test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    async fn test_all_releases_macos_aarch64() {
        assert!(all_releases().await.is_ok());
    }

    #[tokio::test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    async fn test_all_releases_linux_amd64() {
        assert!(all_releases().await.is_ok());
    }

    #[tokio::test]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    async fn test_all_releases_linux_aarch64() {
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
