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
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

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
pub use platform::{platform, Platform};

mod releases;
pub use releases::{all_releases, Releases};

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
        let v = v?;
        let path = v.path();
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if file_name == ".global-version" {
            continue;
        }
        versions.push(Version::parse(file_name)?);
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
