use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! declare_target {
    ($os:ident, $arch:ident, $os_str:literal, $arch_str:literal) => {
        #[cfg(all(target_os = $os_str, target_arch = $arch_str))]
        use concat_idents::concat_idents;
        #[cfg(all(target_os = $os_str, target_arch = $arch_str))]
        concat_idents!(mod_name = $os, _, $arch {
            mod mod_name;
        });
        #[cfg(all(target_os = $os_str, target_arch = $arch_str))]
        concat_idents!(mod_name = $os, _, $arch {
            pub use mod_name::{all_releases, artifact_url};
        });
        #[cfg(all(feature = "blocking", target_os = $os_str, target_arch = $arch_str))]
        concat_idents!(mod_name = $os, _, $arch {
            pub use mod_name::blocking_all_releases;
        });
    };
}

declare_target!(linux, x86_64, "linux", "x86_64");
declare_target!(linux, aarch64, "linux", "aarch64");
declare_target!(macos, x86_64, "macos", "x86_64");
declare_target!(macos, aarch64, "macos", "aarch64");
declare_target!(windows, x86_64, "windows", "x86_64");

mod util;
use util::hex_string;

/// Prefix to the URLs to fetch solc metadata and the solc binaries.
///
/// List URL  : {SOLC_RELEASES_URL}/{platform}/list.json
/// Binary URL: {SOLC_RELEASES_URL}/{platform}/{artifact}
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
    /// List of `BuildInfo`.
    pub builds: Vec<BuildInfo>,
    /// Map of version to artifact.
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
    /// Solc version.
    pub version: Version,
    /// Expected SHA-256 checksum of the solc binary.
    #[serde(with = "hex_string")]
    pub sha256: Vec<u8>,
}
