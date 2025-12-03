#![allow(dead_code)]

use std::fs::{self, File};
use std::path::PathBuf;

use semver::{BuildMetadata, Prerelease, Version};
use svm::Releases;

/// The string describing the [svm::Platform] to build for
///
/// Supported values are:
///
/// - "linux-amd64"
/// - "linux-aarch64"
/// - "macosx-amd64"
/// - "macosx-aarch64"
/// - "windows-amd64"
/// - "android-aarch64"
pub const SVM_TARGET_PLATFORM: &str = "SVM_TARGET_PLATFORM";

/// The path to the releases JSON file, that was pre-fetched manually.
///
/// If this variable is defined, svm-builds won't attempt to deduce anything about the
/// platfrom or perform network calls, and will instead treat the file as the
/// source of truth.
///
/// Must follow the same format as the upstream `list.json`, e.g.
/// [this one](https://raw.githubusercontent.com/nikitastupin/solc/af2fce8988e41753ab4f726e0273ea8244de5dba/linux/aarch64/list.json)
pub const SVM_RELEASES_LIST_JSON: &str = "SVM_RELEASES_LIST_JSON";

/// Returns the platform to generate the constants for
///
/// if the `SVM_TARGET_PLATFORM` var is set, this will return the matching [svm::Platform],
/// otherwise the native platform will be used [svm::platform()].
fn get_platform() -> svm::Platform {
    if let Ok(s) = std::env::var(SVM_TARGET_PLATFORM) {
        s.parse().unwrap()
    } else {
        svm::platform()
    }
}

fn version_const_name(version: &Version) -> String {
    if version.pre == Prerelease::EMPTY {
        format!(
            "SOLC_VERSION_{}_{}_{}",
            version.major, version.minor, version.patch
        )
    } else {
        let sanitized_pre = version
            .pre
            .as_str()
            .replace(|c: char| !c.is_alphanumeric(), "")
            .to_uppercase();
        format!(
            "SOLC_VERSION_{}_{}_{}_{}",
            version.major, version.minor, version.patch, sanitized_pre
        )
    }
}

/// Adds build info related constants
fn add_build_info_constants(output: &mut String, releases: &Releases, platform: svm::Platform) {
    let mut version_idents = Vec::with_capacity(releases.builds.len());
    let mut checksum_match_arms = Vec::with_capacity(releases.builds.len());
    for build in releases.builds.iter() {
        let prerelease = build.prerelease.clone().unwrap_or_default();
        let version = Version {
            major: build.version.major,
            minor: build.version.minor,
            patch: build.version.patch,
            pre: Prerelease::new(&prerelease).unwrap_or_default(),
            build: BuildMetadata::EMPTY,
        };
        let version_name = version_const_name(&version);

        output.push_str(&format!(
            "/// Solidity compiler version `{version}`.\n\
             pub const {version_name}: semver::Version = semver::Version::new({}, {}, {});\n\n",
            version.major, version.minor, version.patch
        ));

        let sha256 = hex::encode(&build.sha256);
        let checksum_name = format!("{version_name}_CHECKSUM");

        version_idents.push(version_name);

        output.push_str(&format!(
            "/// Checksum for Solidity compiler version `{version}`.\n\
             pub const {checksum_name}: &str = \"{sha256}\";\n\n",
        ));
        checksum_match_arms.push(format!(
            "({}, {}, {}, \"{}\") => {}",
            version.major, version.minor, version.patch, version.pre, checksum_name
        ));
    }

    let raw_static_array = format!(
        r#"
/// All available releases for {}
pub static ALL_SOLC_VERSIONS : [semver::Version; {}] = [
    {}
];
"#,
        platform,
        version_idents.len(),
        version_idents.join(",\n    ")
    );
    output.push_str(&raw_static_array);

    let get_check_sum_fn = format!(
        r#"
/// Get the checksum of a solc version's binary if it exists.
pub fn get_checksum(version: &semver::Version) -> Option<Vec<u8>> {{
    let checksum = match (version.major, version.minor, version.patch, version.pre.as_str()) {{
        {},
        _ => return None
    }};
    Some(hex::decode(checksum).unwrap())
}}
"#,
        checksum_match_arms.join(",\n        ")
    );

    output.push_str(&get_check_sum_fn);
}

/// checks the current platform and adds it as constant
fn add_platform_const(output: &mut String, platform: svm::Platform) {
    output.push_str(&format!(
        r#"
/// The `svm::Platform` all constants were built for
pub const TARGET_PLATFORM: &str = "{platform}";
"#
    ));
}

fn generate() {
    let platform = get_platform();
    let out_dir =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));
    let output_file = out_dir.join("generated.rs");

    // Start with an empty string that we'll append to
    let mut output = String::new();

    // Add standard header
    output.push_str("// Generated file, do not edit by hand\n\n");

    // add the platform as constant
    add_platform_const(&mut output, platform);

    let releases: Releases = if let Ok(file_path) = std::env::var(SVM_RELEASES_LIST_JSON) {
        let file = File::open(file_path).unwrap_or_else(|_| {
            panic!("{SVM_RELEASES_LIST_JSON:?} defined, but cannot read the file referenced")
        });

        serde_json::from_reader(file).unwrap_or_else(|_| {
            panic!("Failed to parse the JSON from {SVM_RELEASES_LIST_JSON:?} file")
        })
    } else {
        svm::blocking_all_releases(platform).expect("Failed to fetch releases")
    };

    // add all solc version info
    add_build_info_constants(&mut output, &releases, platform);

    // add the whole release string
    let release_json = serde_json::to_string(&releases).unwrap();
    output.push_str(&format!(
        r##"
/// JSON release list
pub static RELEASE_LIST_JSON : &str = r#"{release_json}"#;"##
    ));

    // Write the string to file
    fs::write(output_file, output).expect("failed to write output file");

    // Tell Cargo that we need to rerun this if any of the relevant env vars change
    println!("cargo:rerun-if-env-changed={SVM_TARGET_PLATFORM}");
    println!("cargo:rerun-if-env-changed={SVM_RELEASES_LIST_JSON}");
}

/// generates an empty `RELEASE_LIST_JSON` static
fn generate_offline() {
    let out_dir =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));
    let output_file = out_dir.join("generated.rs");

    // Start with an empty string
    let mut output = String::new();

    // Add standard header
    output.push_str("// Generated file, do not edit by hand\n\n");

    let release_json = serde_json::to_string(&Releases::default()).unwrap();
    output.push_str(&format!(
        r##"
/// JSON release list
pub static RELEASE_LIST_JSON : &str = r#"{release_json}"#;"##
    ));

    // Write the string to file
    fs::write(output_file, output).expect("failed to write output file");
}

fn main() {
    #[cfg(not(feature = "_offline"))]
    if std::env::var("DOCS_RS").is_ok() {
        // no network access allowed during docs rs builds
        generate_offline();
    } else {
        generate();
    }

    #[cfg(feature = "_offline")]
    generate_offline();
}
