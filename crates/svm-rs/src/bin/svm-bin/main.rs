//! Main svm-rs binary entry point.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use clap::Parser;

mod install;
mod list;
mod print;
mod remove;
mod r#use;
mod utils;
mod which;

/// Solc version manager.
#[derive(Debug, Parser)]
#[clap(
    name = "svm",
    version = svm::VERSION_MESSAGE,
    next_display_order = None,
)]
enum Svm {
    List(list::ListCmd),
    Install(install::InstallCmd),
    Use(r#use::UseCmd),
    Which(which::WhichCmd),
    Remove(remove::RemoveCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Svm::parse();

    svm::setup_data_dir()?;

    match opt {
        Svm::List(cmd) => cmd.run().await?,
        Svm::Install(cmd) => cmd.run().await?,
        Svm::Use(cmd) => cmd.run().await?,
        Svm::Which(cmd) => cmd.run()?,
        Svm::Remove(cmd) => cmd.run().await?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Svm::command().debug_assert();
    }
}
