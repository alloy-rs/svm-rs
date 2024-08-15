//! Simple Solc wrapper that delegates everything to a specified or the global [`svm`] Solc version.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use anyhow::Context;
use std::io;
use std::process::{Command, ExitStatus, Stdio};

fn main() {
    let code = match main_() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("svm: error: {err:?}");
            1
        }
    };
    std::process::exit(code);
}

fn main_() -> anyhow::Result<i32> {
    let mut args = std::env::args_os().skip(1).peekable();
    let version = 'v: {
        // Try to parse the first argument as a version specifier `+x.y.z`.
        if let Some(arg) = args.peek() {
            if let Some(arg) = arg.to_str() {
                if let Some(stripped) = arg.strip_prefix('+') {
                    let version = stripped
                        .parse::<semver::Version>()
                        .context("failed to parse version specifier")?;
                    if !version.build.is_empty() || !version.pre.is_empty() {
                        anyhow::bail!(
                            "version specifier must not have pre-release or build metadata"
                        );
                    }
                    args.next();
                    break 'v version;
                }
            }
        }
        // Fallback to the global version if one is not specified.
        svm::get_global_version()?.ok_or(svm::SvmError::GlobalVersionNotSet)?
    };

    let bin = svm::version_binary(&version.to_string());
    if !bin.exists() {
        anyhow::bail!(
            "Solc version {version} is not installed or does not exist; looked at {}",
            bin.display()
        );
    }

    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    Ok(exec(&mut cmd)?.code().unwrap_or(-1))
}

fn exec(cmd: &mut Command) -> io::Result<ExitStatus> {
    #[cfg(unix)]
    {
        use std::os::unix::prelude::*;
        Err(cmd.exec())
    }
    #[cfg(not(unix))]
    {
        cmd.status()
    }
}
