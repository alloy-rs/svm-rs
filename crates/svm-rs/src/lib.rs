#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(rustdoc::all)]
#![cfg_attr(
    not(any(test, feature = "cli", feature = "solc")),
    warn(unused_crate_dependencies)
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use semver::Version;
use std::fs;

mod error;
pub use error::SvmError;

mod install;
#[cfg(feature = "blocking")]
pub use install::blocking_install;
pub use install::install;

mod paths;
pub use paths::{data_dir, global_version_path, setup_data_dir, version_binary, version_path};

mod platform;
pub use platform::{Platform, platform};

mod releases;
pub use releases::{BuildInfo, Releases, all_releases};

#[cfg(feature = "blocking")]
pub use releases::blocking_all_releases;

#[cfg(feature = "cli")]
#[doc(hidden)]
pub const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

/// Reads the currently set global version for Solc. Returns None if none has yet been set.
pub fn get_global_version() -> Result<Option<Version>, SvmError> {
    let v = fs::read_to_string(global_version_path())?;
    Ok(Version::parse(v.trim_end_matches('\n')).ok())
}

/// Sets the provided version as the global version for Solc.
pub fn set_global_version(version: &Version) -> Result<(), SvmError> {
    fs::write(global_version_path(), version.to_string()).map_err(Into::into)
}

/// Unset the global version. This should be done if all versions are removed.
pub fn unset_global_version() -> Result<(), SvmError> {
    fs::write(global_version_path(), "").map_err(Into::into)
}

/// Reads the list of Solc versions that have been installed in the machine.
/// The version list is sorted in ascending order.
pub fn installed_versions() -> Result<Vec<Version>, SvmError> {
    let mut versions = vec![];
    for v in fs::read_dir(data_dir())? {
        let path = v?.path();
        // Only consider version directories and ignore all other entries, such as the global
        // version file, per-version install lock files or temporary files of installations that
        // are currently in progress.
        if !path.is_dir() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
            continue;
        };
        let Ok(version) = Version::parse(file_name) else {
            continue;
        };
        // Only count fully installed versions: the version directory is created before the binary
        // is downloaded and renamed into place.
        if !version_binary(file_name).is_file() {
            continue;
        }
        versions.push(version);
    }
    versions.sort();
    Ok(versions)
}

/// Blocking version of [`all_versions`]
#[cfg(feature = "blocking")]
pub fn blocking_all_versions() -> Result<Vec<Version>, SvmError> {
    Ok(releases::blocking_all_releases(platform::platform())?.into_versions())
}

/// Fetches the list of all the available versions of Solc. The list is platform dependent, so
/// different versions can be found for macosx vs linux.
pub async fn all_versions() -> Result<Vec<Version>, SvmError> {
    Ok(releases::all_releases(platform::platform())
        .await?
        .into_versions())
}

/// Removes the provided version of Solc from the machine.
///
/// Note: removing a version that is concurrently being installed or executed is inherently racy;
/// this also removes the version's install lock file, so an installation that is in progress at
/// the same time can fail or reinstall the version.
pub fn remove_version(version: &Version) -> Result<(), SvmError> {
    fs::remove_dir_all(version_path(version.to_string().as_str())).map_err(Into::into)
}

fn setup_version(version: &str) -> Result<(), SvmError> {
    let v = version_path(version);
    if !v.exists() {
        fs::create_dir_all(v)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Auxiliary entries in the data directory, such as lock files left behind by older versions,
    /// must not fail the version listing, and only fully installed versions are listed.
    #[test]
    fn installed_versions_ignores_auxiliary_entries() {
        setup_data_dir().unwrap();
        let dir = data_dir();
        fs::write(dir.join(".lock-solc-0.8.10"), "").unwrap();
        fs::write(dir.join(".tmpXYZ123"), "").unwrap();
        fs::write(dir.join(".DS_Store"), "").unwrap();
        for version in ["0.8.10", "0.8.24"] {
            fs::create_dir_all(version_path(version)).unwrap();
            fs::write(version_binary(version), "solc").unwrap();
        }
        // A version directory without a binary is an installation that never completed.
        fs::create_dir_all(version_path("99.99.99")).unwrap();

        let versions = installed_versions().unwrap();
        assert!(versions.contains(&Version::new(0, 8, 10)));
        assert!(versions.contains(&Version::new(0, 8, 24)));
        assert!(!versions.contains(&Version::new(99, 99, 99)));
    }
}
