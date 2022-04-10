use semver::Version;
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
pub const SVM_TARGET_PLATFORM: &str = "SVM_TARGET_PLATFORM";

/// Returns the platform to generate the constants for
///
/// if the `SVM_TARGET_PLATFORM` var is set, this will return the matching [svm::Platform], otherwise the native platform will be used [svm::platform()]
fn get_platform() -> svm::Platform {
    if let Ok(s) = std::env::var(SVM_TARGET_PLATFORM) {
        s.parse().unwrap()
    } else {
        svm::platform()
    }
}

fn version_const_name(version: &Version) -> String {
    format!(
        "SOLC_VERSION_{}_{}_{}",
        version.major, version.minor, version.patch
    )
}

/// Adds build info related constants
fn add_build_info_constants(
    writer: &mut build_const::ConstValueWriter,
    releases: &Releases,
    platform: svm::Platform,
) {
    let mut version_idents = Vec::with_capacity(releases.builds.len());
    let mut checksum_match_arms = Vec::with_capacity(releases.builds.len());

    for build in releases.builds.iter() {
        let version_name = version_const_name(&build.version);

        writer.add_value_raw(
            &version_name,
            "semver::Version",
            &format!(
                "semver::Version::new({},{},{})",
                build.version.major, build.version.minor, build.version.patch
            ),
        );
        version_idents.push(version_name);

        let sha256 = hex::encode(&build.sha256);
        let checksum_name = format!(
            "SOLC_VERSION_{}_{}_{}_CHECKSUM",
            build.version.major, build.version.minor, build.version.patch
        );

        writer.add_value(&checksum_name, "&str", sha256);
        checksum_match_arms.push(format!(
            "({},{},{})  => {}",
            build.version.major, build.version.minor, build.version.patch, checksum_name
        ));
    }

    let raw_static_array = format!(
        r#"
/// All available releases for {}
pub static ALL_SOLC_VERSIONS : [semver::Version; {}] = [
    {}  ];
    "#,
        platform,
        version_idents.len(),
        version_idents.join(",\n")
    );
    writer.add_raw(&raw_static_array);

    let get_check_sum_fn = format!(
        r#"
/// Get the checksum of a solc version's binary if it exists.
pub fn get_checksum(version: &semver::Version) -> Option<Vec<u8>> {{
    let checksum = match (version.major, version.minor, version.patch) {{
        {},
        _ => return None
    }};
    Some(hex::decode(checksum).expect("valid hex;"))
}}
    "#,
        checksum_match_arms.join(",\n")
    );

    writer.add_raw(&get_check_sum_fn);
}

/// checks the current platform and adds it as constant
fn add_platform_const(writer: &mut build_const::ConstValueWriter, platform: svm::Platform) {
    writer.add_raw(&format!(
        r#"
/// The `svm::Platform` all constants were built for
pub const TARGET_PLATFORM: &str = "{}";
"#,
        platform
    ));
}

fn main() {
    let platform = get_platform();
    let releases = svm::blocking_all_releases().expect("Failed to fetch releases");

    let mut writer = build_const::ConstWriter::for_build("builds")
        .unwrap()
        .finish_dependencies();

    // add the platform as constant
    add_platform_const(&mut writer, platform);

    // add all solc version info
    add_build_info_constants(&mut writer, &releases, platform);

    // add the whole release string
    let release_json = serde_json::to_string(&releases).unwrap();
    writer.add_raw(&format!(
        r#"
/// JSON release list
pub static RELEASE_LIST_JSON : &str = {}"{}"{};"#,
        "r#", release_json, "#"
    ));

    writer.finish();
}
