use once_cell::sync::Lazy;
use reqwest::get;
use semver::Version;
use url::Url;

use super::{Releases, SOLC_RELEASES_URL};
use crate::{error::SolcVmError, platform::Platform};

/// Version from which we support solc binaries natively built for Platform::MacOsAarch64.
static NATIVE_BUILDS_FROM: Lazy<Version> = Lazy::new(|| Version::new(0, 8, 5));

/// Prefix URL for downloading solc binaries natively built for Platform::MacOsAarch64.
///
/// Binary URL: {URL_PREFIX}/{artifact}
static URL_PREFIX: &str =
    "https://github.com/roynalnaruto/solc-builds/raw/465839dcbb23fd4e60c16e8cae32513cd5627ca0/macosx/aarch64";

/// URL for fetching the metadata (release info) of solc binaries natively built for
/// Platform::MacOsAarch64.
static RELEASES_URL: &str =
    "https://github.com/roynalnaruto/solc-builds/raw/465839dcbb23fd4e60c16e8cae32513cd5627ca0/macosx/aarch64/list.json";

/// The supported versions for both macos-amd64 and macos-aarch64 are the same.
///
/// 1. For version >= 0.8.5 we fetch native releases from
/// https://github.com/roynalnaruto/solc-builds
///
/// 2. For version <= 0.8.4 we fetch releases from https://binaries.soliditylang.org and
/// require Rosetta support.
#[cfg(feature = "blocking")]
pub fn blocking_all_releases() -> Result<Releases, SolcVmError> {
    let mut native = reqwest::blocking::get(RELEASES_URL)?.json::<Releases>()?;
    let mut releases = reqwest::blocking::get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        Platform::MacOsAmd64,
    ))?
    .json::<Releases>()?;
    releases.builds = releases
        .builds
        .iter()
        .filter(|b| b.version.lt(&NATIVE_BUILDS_FROM))
        .cloned()
        .collect();
    releases.builds.extend_from_slice(&native.builds);
    releases.releases.append(&mut native.releases);
    Ok(releases)
}

/// The supported versions for both macos-amd64 and macos-aarch64 are the same.
///
/// 1. For version >= 0.8.5 we fetch native releases from
/// https://github.com/roynalnaruto/solc-builds
///
/// 2. For version <= 0.8.4 we fetch releases from https://binaries.soliditylang.org and
/// require Rosetta support.
pub async fn all_releases() -> Result<Releases, SolcVmError> {
    let mut native = get(RELEASES_URL).await?.json::<Releases>().await?;
    let mut releases = get(format!(
        "{}/{}/list.json",
        SOLC_RELEASES_URL,
        Platform::MacOsAmd64,
    ))
    .await?
    .json::<Releases>()
    .await?;
    releases.builds = releases
        .builds
        .iter()
        .filter(|b| b.version.lt(&NATIVE_BUILDS_FROM))
        .cloned()
        .collect();
    releases.builds.extend_from_slice(&native.builds);
    releases.releases.append(&mut native.releases);
    Ok(releases)
}

/// Constructs the URL to the solc binary with the given version and artifact for
/// Platform::MacOsAarch64.
pub fn artifact_url(version: &Version, artifact: &str) -> Result<Url, SolcVmError> {
    if version.ge(&NATIVE_BUILDS_FROM) {
        Ok(Url::parse(&format!("{}/{}", URL_PREFIX, artifact))?)
    } else {
        Ok(Url::parse(&format!(
            "{}/{}/{}",
            SOLC_RELEASES_URL,
            Platform::MacOsAmd64,
            artifact,
        ))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_macos_aarch64() {
        let releases = all_releases()
            .await
            .expect("could not fetch releases for macos-aarch64");
        let rosetta = Version::new(0, 8, 4);
        let native = NATIVE_BUILDS_FROM.clone();
        let url1 = artifact_url(&rosetta, releases.get_artifact(&rosetta).unwrap())
            .expect("could not fetch artifact URL");
        let url2 = artifact_url(&native, releases.get_artifact(&native).unwrap())
            .expect("could not fetch artifact URL");
        assert!(url1.to_string().contains(SOLC_RELEASES_URL));
        assert!(url2.to_string().contains(URL_PREFIX));
    }

    #[tokio::test]
    async fn test_all_releases_macos_aarch64() {
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
