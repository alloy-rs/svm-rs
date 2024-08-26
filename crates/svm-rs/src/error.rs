use reqwest::StatusCode;
use thiserror::Error;
use url::Url;

/// Error types from the svm_lib crate.
#[derive(Debug, Error)]
pub enum SvmError {
    #[error("SVM global version not set")]
    GlobalVersionNotSet,
    #[error("Unknown version provided")]
    UnknownVersion,
    #[error("Unsupported version {0} for platform {1}")]
    UnsupportedVersion(String, String),
    #[error("Version {0} not installed")]
    VersionNotInstalled(String),
    #[error("Checksum mismatch for version {version}: expected: {expected}, actual: {actual}")]
    ChecksumMismatch {
        version: String,
        expected: String,
        actual: String,
    },
    #[error("Install step for solc version {0} timed out after {1} seconds")]
    Timeout(String, u64),
    #[error("Unable to patch solc binary for nixos. stdout: {0}. stderr: {1}")]
    CouldNotPatchForNixOs(String, String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    PersistError(#[from] tempfile::PathPersistError),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SemverError(#[from] semver::Error),
    #[error(transparent)]
    UrlError(#[from] url::ParseError),
    #[error("Received unsuccessful response with code {1} for {0}")]
    UnsuccessfulResponse(Url, StatusCode),
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    #[error(transparent)]
    ZipError(#[from] zip::result::ZipError),
}
