use thiserror::Error;

/// Various error types emitted/propagated by SVM.
#[derive(Debug, Error)]
pub enum SolcVmError {
    /// Global version is not set in the current machine. Global version can be found in `~/.svm/.global-version`.
    #[error("SVM global version not set")]
    GlobalVersionNotSet,
    /// Unknown version provided to SVM.
    #[error("Unknown version provided")]
    UnknownVersion,
    /// Provided version is not supported for the platform.
    #[error("Unsupported version {0} for platform {1}")]
    UnsupportedVersion(String, String),
    /// Version is not installed in the current machine.
    #[error("Version {0} not installed")]
    VersionNotInstalled(String),
    /// SHA-256 checksum of the `solc` binary for the given version does not match the expected
    /// checksum.
    #[error("Checksum mismatch for version {0}")]
    ChecksumMismatch(String),
    /// Installation timed out.
    #[error("Installation of solc version {0} timed out after {1} seconds")]
    Timeout(String, u64),
    /// Error propagated from `std::io`.
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    /// Error propagated from `reqwest`.
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    /// Error propagated from `semver`.
    #[error(transparent)]
    SemverError(#[from] semver::Error),
    /// Error propagated from `url`.
    #[error(transparent)]
    UrlError(#[from] url::ParseError),
}
