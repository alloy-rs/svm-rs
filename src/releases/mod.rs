use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux_x86_64;
#[cfg(all(feature = "blocking", target_os = "linux", target_arch = "x86_64"))]
pub use linux_x86_64::blocking_all_releases;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use linux_x86_64::{all_releases, artifact_url};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
mod linux_aarch64;
#[cfg(all(feature = "blocking", target_os = "linux", target_arch = "aarch64"))]
pub use linux_aarch64::blocking_all_releases;
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub use linux_aarch64::{all_releases, artifact_url};

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
mod macos_x86_64;
#[cfg(all(feature = "blocking", target_os = "macos", target_arch = "x86_64"))]
pub use macos_x86_64::blocking_all_releases;
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub use macos_x86_64::{all_releases, artifact_url};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod macos_aarch64;
#[cfg(all(feature = "blocking", target_os = "macos", target_arch = "aarch64"))]
pub use macos_aarch64::blocking_all_releases;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use macos_aarch64::{all_releases, artifact_url};

mod util;
use util::hex_string;

const SOLC_RELEASES_URL: &str = "https://binaries.soliditylang.org";

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
