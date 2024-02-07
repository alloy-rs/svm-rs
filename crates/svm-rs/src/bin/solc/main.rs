//! Simple Solc wrapper that delegates everything to the global [`svm`] version.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use std::process::{Command, Stdio};

fn main() -> anyhow::Result<()> {
    let version = svm::get_global_version()?.ok_or(svm::SvmError::GlobalVersionNotSet)?;
    let program = svm::version_binary(&version.to_string());
    let status = Command::new(program)
        .args(std::env::args_os().skip(1))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    let code = status.code().unwrap_or(-1);
    std::process::exit(code);
}
