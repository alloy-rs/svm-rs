//! Contains all Solc builds for the platform it was compiled with.

// Only used by the generated code, which is empty with the `_offline` feature.
use const_hex as _;
use semver as _;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));
