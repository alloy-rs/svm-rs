//! Simple Solc wrapper that delegates everything to a specified or the global [`svm`] Solc version.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use anyhow::Context;
use std::io;
use std::process::{Command, ExitStatus, Stdio};

fn main() {
    let code = match main_() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("svm: error: {err}");
            1
        }
    };
    std::process::exit(code);
}

fn main_() -> anyhow::Result<i32> {
    let mut args = std::env::args_os().skip(1).peekable();

    // Try to parse the first argument as a version specifier `+x.y.z`.
    let version = if let Some(arg) = args.peek()
        && let Some(arg) = arg.to_str()
        && let Some(stripped) = arg.strip_prefix('+')
    {
        let version = stripped
            .parse::<semver::Version>()
            .context("failed to parse version specifier")?;
        if !version.build.is_empty() || !version.pre.is_empty() {
            anyhow::bail!("version specifier must not have pre-release or build metadata");
        }
        args.next();
        version
    } else {
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
        retry_busy(|| Err(cmd.exec()))
    }
    #[cfg(not(unix))]
    {
        // Retry process creation only, never wait errors or the compiler's exit status.
        retry_busy(|| cmd.spawn())?.wait()
    }
}

// Closing the installer's write handle does not close duplicates inherited by concurrently forked
// children. Retry only a rejected launch, for at most 310 ms; a successfully executed program is
// never rerun. This protects the svm solc wrapper, not consumers launching cached solc directly.
fn retry_busy<T>(mut launch: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    for delay in [10, 20, 40, 80, 160] {
        match launch() {
            Err(err) if err.kind() == io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            result => return result,
        }
    }
    launch()
}

#[cfg(test)]
mod tests;
