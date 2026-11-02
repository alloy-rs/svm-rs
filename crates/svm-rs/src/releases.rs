use crate::{error::SvmError, platform::Platform};
use reqwest::StatusCode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};
use url::Url;

// Updating new releases:
// 1. Update `https://github.com/nikitastupin/solc` commit for `linux/aarch64`
// 2. Update LATEST for tests

/// Base URL for all Solc releases
/// `"SOLC_RELEASES_URL}/{platform}/list.json"`:
/// `https://binaries.soliditylang.org/linux-amd64/list.json`
/// `https://binaries.soliditylang.org/windows-amd64/list.json`
/// `https://binaries.soliditylang.org/macosx-amd64/list.json`
const SOLC_RELEASES_URL: &str = "https://binaries.soliditylang.org";

const OLD_SOLC_RELEASES_DOWNLOAD_PREFIX: &str =
    "https://raw.githubusercontent.com/crytic/solc/master/linux/amd64";

const OLD_VERSION_MAX: Version = Version::new(0, 4, 9);

const OLD_VERSION_MIN: Version = Version::new(0, 4, 0);

static OLD_SOLC_RELEASES: LazyLock<Releases> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../list/linux-arm64-old.json"))
        .expect("could not parse list linux-arm64-old.json")
});

const LINUX_AARCH64_MIN: Version = Version::new(0, 5, 0);

// NOTE: Since version 0.8.31, Linux aarch64 releases are available: https://binaries.soliditylang.org/linux-arm64/list.json
const LINUX_AARCH64_BINARIES: Version = Version::new(0, 8, 31);

// NOTE: The official Solidity releases uses the platform name "linux-arm64"
// instead of the "linux-aarch64" naming which is used by SVM.
static LINUX_AARCH64_PLATFORM: &str = "linux-arm64";

static LINUX_AARCH64_URL_PREFIX: &str = "https://raw.githubusercontent.com/nikitastupin/solc/2287d4326237172acf91ce42fd7ec18a67b7f512/linux/aarch64";

static LINUX_AARCH64_RELEASES_URL: &str = "https://raw.githubusercontent.com/nikitastupin/solc/2287d4326237172acf91ce42fd7ec18a67b7f512/linux/aarch64/list.json";

// NOTE: Since version 0.8.24, universal macosx releases are available: https://binaries.soliditylang.org/macosx-amd64/list.json
const MACOS_AARCH64_NATIVE: Version = Version::new(0, 8, 5);

const UNIVERSAL_MACOS_BINARIES: Version = Version::new(0, 8, 24);

static MACOS_AARCH64_URL_PREFIX: &str = "https://raw.githubusercontent.com/alloy-rs/solc-builds/e4b80d33bc4d015b2fc3583e217fbf248b2014e1/macosx/aarch64";

static MACOS_AARCH64_RELEASES_URL: &str = "https://raw.githubusercontent.com/alloy-rs/solc-builds/e4b80d33bc4d015b2fc3583e217fbf248b2014e1/macosx/aarch64/list.json";

const ANDROID_AARCH64_MIN: Version = Version::new(0, 8, 24);

static ANDROID_AARCH64_URL_PREFIX: &str = "https://raw.githubusercontent.com/alloy-rs/solc-builds/ac6f303a04b38e7ec507ced511fd3ed7a605179f/android/aarch64";

static ANDROID_AARCH64_RELEASES_URL: &str = "https://raw.githubusercontent.com/alloy-rs/solc-builds/ac6f303a04b38e7ec507ced511fd3ed7a605179f/android/aarch64/list.json";

/// Defines the struct that the JSON-formatted release list can be deserialized into.
///
/// Both the key and value are deserialized into [`semver::Version`].
///
/// ```json
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
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Releases {
    pub builds: Vec<BuildInfo>,
    pub releases: BTreeMap<Version, String>,
}

impl Releases {
    /// Get the checksum of a solc version's binary if it exists.
    /// Checks for exact version match or for prerelease.
    pub fn get_checksum(&self, v: &Version) -> Option<Vec<u8>> {
        let matches = |build_info: &BuildInfo| {
            build_info.version == Version::new(v.major, v.minor, v.patch)
                && build_info
                    .prerelease
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(v.pre.as_str())
        };

        self.builds
            .iter()
            .find(|build_info| matches(build_info))
            .map(|build_info| build_info.sha256.clone())
    }

    /// Returns the artifact of the version if any, by looking it up in releases or in builds (if
    /// a prerelease).
    pub fn get_artifact(&self, version: &Version) -> Option<&String> {
        // Check version artifact in releases.
        if let Some(artifact) = self.releases.get(version) {
            return Some(artifact);
        }

        // If we didn't find any artifact under releases, look up builds for prerelease.
        if !version.pre.is_empty()
            && let Some(build_info) = self.builds.iter().find(|b| {
                b.version == Version::new(version.major, version.minor, version.patch)
                    && b.prerelease == Some(version.pre.to_string())
            })
        {
            return build_info.path.as_ref();
        }

        None
    }

    /// Returns a sorted list of all versions
    pub fn into_versions(self) -> Vec<Version> {
        let mut versions = self.releases.into_keys().collect::<Vec<_>>();
        versions.sort_unstable();
        versions
    }

    /// Retain versions matching a predicate.
    fn retain_versions(&mut self, mut pred: impl FnMut(&Version) -> bool) {
        self.builds.retain(|build| pred(&build.version));
        self.releases.retain(|version, _| pred(version));
    }

    /// Extends releases with another.
    fn extend(&mut self, other: Self) {
        self.builds.extend(other.builds);
        self.releases.extend(other.releases);
    }
}

/// Build info contains the SHA256 checksum of a solc binary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: Version,
    #[serde(with = "hex_string")]
    pub sha256: Vec<u8>,
    pub path: Option<String>,
    pub prerelease: Option<String>,
}

/// Helper serde module to serialize and deserialize bytes as hex.
mod hex_string {
    use super::*;
    use serde::{Deserializer, Serializer, de};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        hex::decode(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: AsRef<[u8]>,
    {
        serializer.serialize_str(&hex::encode_prefixed(value))
    }
}

/// How release lists are fetched: per-request timeout and retries for transient failures.
#[derive(Clone, Copy, Debug)]
struct FetchPolicy {
    /// Timeout for a single request, including reading the body.
    timeout: Duration,
    /// Total number of attempts, including the first one.
    attempts: u32,
    /// Delay before the first retry; doubled after each failed attempt.
    retry_delay: Duration,
}

const FETCH_POLICY: FetchPolicy = FetchPolicy {
    timeout: Duration::from_secs(30),
    attempts: 4,
    retry_delay: Duration::from_secs(1),
};

impl FetchPolicy {
    /// Returns the delay before retrying after the given failed attempt (0-based).
    fn backoff(&self, attempt: u32) -> Duration {
        self.retry_delay.saturating_mul(1 << attempt.min(16))
    }
}

/// Returns `true` if a request that failed with this status may succeed when retried.
fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Returns `true` if a request that failed with this error may succeed when retried.
///
/// Decoding errors on a successful response are not retried.
fn is_retryable_error(err: &SvmError) -> bool {
    match err {
        SvmError::UnsuccessfulResponse(_, status) => is_retryable_status(*status),
        SvmError::ReqwestError(err) => err.is_connect() || err.is_timeout() || err.is_request(),
        _ => false,
    }
}

/// Fetches and decodes a release list, retrying transient failures.
async fn fetch_releases(url: &str) -> Result<Releases, SvmError> {
    fetch_releases_with(url, FETCH_POLICY).await
}

async fn fetch_releases_with(url: &str, policy: FetchPolicy) -> Result<Releases, SvmError> {
    let client = reqwest::Client::builder().timeout(policy.timeout).build()?;
    let mut attempt = 0;
    loop {
        let result = async {
            let res = client.get(url).send().await?;
            let status = res.status();
            if !status.is_success() {
                return Err(SvmError::UnsuccessfulResponse(res.url().clone(), status));
            }
            Ok(res.json::<Releases>().await?)
        }
        .await;
        match result {
            Err(err) if attempt + 1 < policy.attempts && is_retryable_error(&err) => {
                tokio::time::sleep(policy.backoff(attempt)).await;
                attempt += 1;
            }
            result => return result,
        }
    }
}

/// Blocking version of [`fetch_releases`].
#[cfg(feature = "blocking")]
fn blocking_fetch_releases(url: &str) -> Result<Releases, SvmError> {
    blocking_fetch_releases_with(url, FETCH_POLICY)
}

#[cfg(feature = "blocking")]
fn blocking_fetch_releases_with(url: &str, policy: FetchPolicy) -> Result<Releases, SvmError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(policy.timeout)
        .build()?;
    let mut attempt = 0;
    loop {
        let result = (|| {
            let res = client.get(url).send()?;
            let status = res.status();
            if !status.is_success() {
                return Err(SvmError::UnsuccessfulResponse(res.url().clone(), status));
            }
            Ok(res.json::<Releases>()?)
        })();
        match result {
            Err(err) if attempt + 1 < policy.attempts && is_retryable_error(&err) => {
                std::thread::sleep(policy.backoff(attempt));
                attempt += 1;
            }
            result => return result,
        }
    }
}

/// Blocking version of [`all_releases`].
#[cfg(feature = "blocking")]
pub fn blocking_all_releases(platform: Platform) -> Result<Releases, SvmError> {
    match platform {
        Platform::LinuxAarch64 => {
            // Prior to version 0.8.31, releases for Linux arm64 builds was provided by the
            // `nikitastupin/solc` repository. From 0.8.31 (inclusive) and onwards, official
            // binary releases are provided by the Solidity project.
            let mut releases = blocking_fetch_releases(LINUX_AARCH64_RELEASES_URL)?;
            releases.retain_versions(|v| *v < LINUX_AARCH64_BINARIES);
            fix_build_prerelease(&mut releases);
            let mut official = blocking_fetch_releases(&format!(
                "{SOLC_RELEASES_URL}/{LINUX_AARCH64_PLATFORM}/list.json",
            ))?;
            // Filtering older releases isn't strinctly necessary here, but do it just in case
            // Solidity retroactively starts adding older release binaries for linux-aarch64.
            official.retain_versions(|v| *v >= LINUX_AARCH64_BINARIES);
            releases.extend(official);
            Ok(releases)
        }
        Platform::MacOsAarch64 => {
            // The supported versions for both macos-amd64 and macos-aarch64 are the same.
            //
            // 1. For version >= 0.8.5 we fetch native releases from
            // https://github.com/alloy-rs/solc-builds
            //
            // 2. For version <= 0.8.4 we fetch releases from https://binaries.soliditylang.org and
            // require Rosetta support.
            //
            // Note: Since 0.8.24 universal macosx releases are available
            let native = blocking_fetch_releases(MACOS_AARCH64_RELEASES_URL)?;
            let mut releases = blocking_fetch_releases(&format!(
                "{}/{}/list.json",
                SOLC_RELEASES_URL,
                Platform::MacOsAmd64,
            ))?;
            releases
                .retain_versions(|v| *v < MACOS_AARCH64_NATIVE || *v > UNIVERSAL_MACOS_BINARIES);
            releases.extend(native);
            Ok(releases)
        }
        Platform::AndroidAarch64 => blocking_fetch_releases(ANDROID_AARCH64_RELEASES_URL),
        Platform::WindowsAarch64 => {
            // Windows ARM64 uses x64 binaries via emulation
            // Solidity does not provide native ARM64 Windows binaries
            let releases = blocking_fetch_releases(&format!(
                "{SOLC_RELEASES_URL}/{}/list.json",
                Platform::WindowsAmd64
            ))?;
            Ok(unified_releases(releases, platform))
        }
        _ => {
            let releases =
                blocking_fetch_releases(&format!("{SOLC_RELEASES_URL}/{platform}/list.json"))?;
            Ok(unified_releases(releases, platform))
        }
    }
}

/// Fetch all releases available for the provided platform.
pub async fn all_releases(platform: Platform) -> Result<Releases, SvmError> {
    match platform {
        Platform::LinuxAarch64 => {
            // Prior to version 0.8.31, releases for Linux arm64 builds was provided by the
            // `nikitastupin/solc` repository. From 0.8.31 (inclusive) and onwards, official
            // binary releases are provided by the Solidity project.
            let mut releases = fetch_releases(LINUX_AARCH64_RELEASES_URL).await?;
            releases.retain_versions(|v| *v < LINUX_AARCH64_BINARIES);
            fix_build_prerelease(&mut releases);
            let mut official = fetch_releases(&format!(
                "{SOLC_RELEASES_URL}/{LINUX_AARCH64_PLATFORM}/list.json",
            ))
            .await?;
            // Filtering older releases isn't strinctly necessary here, but do it just in case
            // Solidity retroactively starts adding older release binaries for linux-aarch64.
            official.retain_versions(|v| *v >= LINUX_AARCH64_BINARIES);
            releases.extend(official);
            Ok(releases)
        }
        Platform::MacOsAarch64 => {
            // The supported versions for both macos-amd64 and macos-aarch64 are the same.
            //
            // 1. For version >= 0.8.5 we fetch native releases from
            // https://github.com/alloy-rs/solc-builds
            //
            // 2. For version <= 0.8.4 we fetch releases from https://binaries.soliditylang.org and
            // require Rosetta support.
            let native = fetch_releases(MACOS_AARCH64_RELEASES_URL).await?;
            let mut releases = fetch_releases(&format!(
                "{}/{}/list.json",
                SOLC_RELEASES_URL,
                Platform::MacOsAmd64,
            ))
            .await?;
            releases
                .retain_versions(|v| *v < MACOS_AARCH64_NATIVE || *v > UNIVERSAL_MACOS_BINARIES);

            releases.extend(native);
            Ok(releases)
        }
        Platform::AndroidAarch64 => fetch_releases(ANDROID_AARCH64_RELEASES_URL).await,
        Platform::WindowsAarch64 => {
            // Windows ARM64 uses x64 binaries via emulation
            // Solidity does not provide native ARM64 Windows binaries
            let releases = fetch_releases(&format!(
                "{SOLC_RELEASES_URL}/{}/list.json",
                Platform::WindowsAmd64
            ))
            .await?;

            Ok(unified_releases(releases, platform))
        }
        _ => {
            let releases =
                fetch_releases(&format!("{SOLC_RELEASES_URL}/{platform}/list.json")).await?;

            Ok(unified_releases(releases, platform))
        }
    }
}

/// unifies the releases with old releases if on linux
fn unified_releases(releases: Releases, platform: Platform) -> Releases {
    if platform == Platform::LinuxAmd64 {
        let mut all_releases = OLD_SOLC_RELEASES.clone();
        all_releases.extend(releases);
        all_releases
    } else {
        releases
    }
}

/// Fixes build pre-release info for certain release lists.
///
/// In particular, the release list from the `nikitastupin/solc` repository does correctly label
/// `prerelease` information in the build array, so populate it based on the `version`.
fn fix_build_prerelease(releases: &mut Releases) {
    for build in &mut releases.builds {
        build.prerelease = if build.version.pre.is_empty() {
            None
        } else {
            Some(build.version.pre.as_str().to_owned())
        };
    }
}

/// Construct the URL to the Solc binary for the specified release version and target platform.
pub(crate) fn artifact_url(
    platform: Platform,
    version: &Version,
    artifact: &str,
) -> Result<Url, SvmError> {
    if platform == Platform::LinuxAmd64
        && *version <= OLD_VERSION_MAX
        && *version >= OLD_VERSION_MIN
    {
        return Ok(Url::parse(&format!(
            "{OLD_SOLC_RELEASES_DOWNLOAD_PREFIX}/{artifact}"
        ))?);
    }

    if platform == Platform::LinuxAarch64 {
        if *version >= LINUX_AARCH64_BINARIES {
            return Ok(Url::parse(&format!(
                "{SOLC_RELEASES_URL}/{LINUX_AARCH64_PLATFORM}/{artifact}"
            ))?);
        } else if *version >= LINUX_AARCH64_MIN {
            return Ok(Url::parse(&format!(
                "{LINUX_AARCH64_URL_PREFIX}/{artifact}"
            ))?);
        } else {
            return Err(SvmError::UnsupportedVersion(
                version.to_string(),
                platform.to_string(),
            ));
        }
    }

    if platform == Platform::MacOsAmd64 && *version < OLD_VERSION_MIN {
        return Err(SvmError::UnsupportedVersion(
            version.to_string(),
            platform.to_string(),
        ));
    }

    if platform == Platform::MacOsAarch64 {
        if *version >= MACOS_AARCH64_NATIVE && *version <= UNIVERSAL_MACOS_BINARIES {
            // fetch natively build solc binaries from `https://github.com/alloy-rs/solc-builds`
            return Ok(Url::parse(&format!(
                "{MACOS_AARCH64_URL_PREFIX}/{artifact}"
            ))?);
        } else {
            // if version is older or universal macos binaries are available, fetch from `https://binaries.soliditylang.org`
            return Ok(Url::parse(&format!(
                "{}/{}/{}",
                SOLC_RELEASES_URL,
                Platform::MacOsAmd64,
                artifact,
            ))?);
        }
    }

    if platform == Platform::AndroidAarch64 {
        if version.ge(&ANDROID_AARCH64_MIN) {
            return Ok(Url::parse(&format!(
                "{ANDROID_AARCH64_URL_PREFIX}/{artifact}"
            ))?);
        } else {
            return Err(SvmError::UnsupportedVersion(
                version.to_string(),
                platform.to_string(),
            ));
        }
    }

    if platform == Platform::WindowsAarch64 {
        // Windows ARM64 uses x64 binaries via emulation
        // Solidity does not provide native ARM64 Windows binaries
        return Ok(Url::parse(&format!(
            "{SOLC_RELEASES_URL}/{}/{artifact}",
            Platform::WindowsAmd64,
        ))?);
    }

    Ok(Url::parse(&format!(
        "{SOLC_RELEASES_URL}/{platform}/{artifact}"
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_url() {
        let version = Version::new(0, 5, 0);
        let artifact = "solc-v0.5.0";
        assert_eq!(
            artifact_url(Platform::LinuxAarch64, &version, artifact).unwrap(),
            Url::parse(&format!(
                "https://raw.githubusercontent.com/nikitastupin/solc/2287d4326237172acf91ce42fd7ec18a67b7f512/linux/aarch64/{artifact}"
            ))
            .unwrap(),
        )
    }

    /// Response served by [`serve`].
    #[derive(Clone)]
    enum Reply {
        /// Respond with this status code and body.
        Status(u16, &'static str),
        /// Read the request and never respond.
        Hang,
    }

    const LIST_JSON: &str = r#"{"builds":[],"releases":{"0.8.0":"solc-v0.8.0"}}"#;

    /// Serves `replies` in order, one per connection, on a local port. Returns the list URL and
    /// a counter of accepted requests.
    fn serve(replies: Vec<Reply>) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
            sync::{Arc, atomic::AtomicUsize, atomic::Ordering},
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/list.json", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let counter = count.clone();
        std::thread::spawn(move || {
            let mut hung = Vec::new();
            for reply in replies {
                let (mut stream, _) = listener.accept().unwrap();
                counter.fetch_add(1, Ordering::SeqCst);
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap() > 2 {
                    line.clear();
                }
                match reply {
                    Reply::Status(code, body) => {
                        let _ = write!(
                            stream,
                            "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\n\
                             content-length: {}\r\nconnection: close\r\n\r\n{body}",
                            body.len()
                        );
                    }
                    Reply::Hang => hung.push(stream),
                }
            }
            // Keep hung connections open until the client gives up.
            std::thread::sleep(Duration::from_secs(5));
            drop(hung);
        });
        (url, count)
    }

    const TEST_POLICY: FetchPolicy = FetchPolicy {
        timeout: Duration::from_millis(500),
        attempts: 3,
        retry_delay: Duration::from_millis(10),
    };

    fn count(c: &std::sync::atomic::AtomicUsize) -> usize {
        c.load(std::sync::atomic::Ordering::SeqCst)
    }

    #[tokio::test]
    async fn fetch_releases_reports_status() {
        let (url, _) = serve(vec![Reply::Status(404, "404")]);
        let err = fetch_releases_with(&url, TEST_POLICY).await.unwrap_err();
        assert!(
            matches!(&err, SvmError::UnsuccessfulResponse(u, s) if u.as_str() == url && *s == StatusCode::NOT_FOUND),
            "{err:?}"
        );
        assert!(err.to_string().contains(&url), "{err}");
    }

    #[tokio::test]
    async fn fetch_releases_retries_rate_limit_and_server_errors() {
        let (url, requests) = serve(vec![
            Reply::Status(429, "429"),
            Reply::Status(503, "503"),
            Reply::Status(200, LIST_JSON),
        ]);
        let releases = fetch_releases_with(&url, TEST_POLICY).await.unwrap();
        assert_eq!(releases.into_versions(), vec![Version::new(0, 8, 0)]);
        assert_eq!(count(&requests), 3);
    }

    #[tokio::test]
    async fn fetch_releases_gives_up_after_attempts() {
        let (url, requests) = serve(vec![Reply::Status(429, "429"); 3]);
        let err = fetch_releases_with(&url, TEST_POLICY).await.unwrap_err();
        assert!(
            matches!(
                err,
                SvmError::UnsuccessfulResponse(_, StatusCode::TOO_MANY_REQUESTS)
            ),
            "{err:?}"
        );
        assert_eq!(count(&requests), 3);
    }

    #[tokio::test]
    async fn fetch_releases_times_out_and_retries() {
        let (url, requests) = serve(vec![Reply::Hang, Reply::Status(200, LIST_JSON)]);
        let releases = fetch_releases_with(&url, TEST_POLICY).await.unwrap();
        assert_eq!(releases.releases.len(), 1);
        assert_eq!(count(&requests), 2);
    }

    #[tokio::test]
    async fn fetch_releases_does_not_retry_client_errors() {
        let (url, requests) = serve(vec![
            Reply::Status(404, "404"),
            Reply::Status(200, LIST_JSON),
        ]);
        assert!(fetch_releases_with(&url, TEST_POLICY).await.is_err());
        assert_eq!(count(&requests), 1);
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn blocking_fetch_releases_retries_rate_limit() {
        let (url, requests) = serve(vec![
            Reply::Status(429, "429"),
            Reply::Status(200, LIST_JSON),
        ]);
        let releases = blocking_fetch_releases_with(&url, TEST_POLICY).unwrap();
        assert_eq!(releases.releases.len(), 1);
        assert_eq!(count(&requests), 2);
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn blocking_fetch_releases_reports_status() {
        let (url, _) = serve(vec![Reply::Status(429, "429"); 3]);
        let err = blocking_fetch_releases_with(&url, TEST_POLICY).unwrap_err();
        assert!(err.to_string().contains("429 Too Many Requests"), "{err}");
        assert!(err.to_string().contains(&url), "{err}");
    }

    #[test]
    fn test_old_releases_deser() {
        assert_eq!(OLD_SOLC_RELEASES.releases.len(), 10);
        assert_eq!(OLD_SOLC_RELEASES.builds.len(), 10);
    }

    #[tokio::test]
    async fn test_macos_aarch64() {
        let releases = all_releases(Platform::MacOsAarch64)
            .await
            .expect("could not fetch releases for macos-aarch64");
        let rosetta = Version::new(0, 8, 4);
        let native = MACOS_AARCH64_NATIVE;
        let url1 = artifact_url(
            Platform::MacOsAarch64,
            &rosetta,
            releases.get_artifact(&rosetta).unwrap(),
        )
        .expect("could not fetch artifact URL");
        let url2 = artifact_url(
            Platform::MacOsAarch64,
            &native,
            releases.get_artifact(&native).unwrap(),
        )
        .expect("could not fetch artifact URL");
        assert!(url1.to_string().contains(SOLC_RELEASES_URL));
        assert!(url2.to_string().contains(MACOS_AARCH64_URL_PREFIX));
    }

    #[tokio::test]
    async fn test_all_releases_macos_amd64() {
        assert!(all_releases(Platform::MacOsAmd64).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_macos_aarch64() {
        assert!(all_releases(Platform::MacOsAarch64).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_linux_amd64() {
        assert!(all_releases(Platform::LinuxAmd64).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_releases_linux_aarch64() {
        let releases = all_releases(Platform::LinuxAarch64)
            .await
            .expect("could not fetch releases for linux-aarch64");
        let thirdparty = LINUX_AARCH64_MIN;
        let url1 = artifact_url(
            Platform::LinuxAarch64,
            &thirdparty,
            releases.get_artifact(&thirdparty).unwrap(),
        )
        .expect("could not fetch artifact URL");
        let prerelease = Version::parse("0.8.31-pre.1").expect("failed to parse version");
        let url2 = artifact_url(
            Platform::LinuxAarch64,
            &prerelease,
            releases.get_artifact(&prerelease).unwrap(),
        )
        .expect("could not fetch artifact URL");
        let official = LINUX_AARCH64_BINARIES;
        let url3 = artifact_url(
            Platform::LinuxAarch64,
            &official,
            releases.get_artifact(&official).unwrap(),
        )
        .expect("could not fetch artifact URL");
        assert!(url1.to_string().contains(LINUX_AARCH64_URL_PREFIX));
        assert!(url2.to_string().contains(LINUX_AARCH64_URL_PREFIX));
        assert!(url3.to_string().contains(SOLC_RELEASES_URL));
    }

    #[tokio::test]
    async fn test_all_releases_windows_aarch64() {
        let releases = all_releases(Platform::WindowsAarch64).await;
        assert!(releases.is_ok());
        // Check also that we got the windows-amd64 release
        let releases = releases.unwrap();
        let latest = releases.releases.keys().max().unwrap();
        let artifact = releases.get_artifact(latest).unwrap();
        let url = artifact_url(Platform::WindowsAarch64, latest, artifact).unwrap();
        assert!(url.to_string().contains("windows-amd64"));
    }

    #[tokio::test]
    async fn releases_roundtrip() {
        let releases = all_releases(Platform::LinuxAmd64).await.unwrap();
        let s = serde_json::to_string(&releases).unwrap();
        let de_releases: Releases = serde_json::from_str(&s).unwrap();
        assert_eq!(releases, de_releases);
    }
}
