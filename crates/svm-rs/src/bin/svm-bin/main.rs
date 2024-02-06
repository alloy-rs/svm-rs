//! Main svm-rs binary entry point.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod install;
mod list;
pub mod print;
mod remove;
mod usev;
mod utils;

use clap::Parser;
use install::InstallArgs;
use list::ListArgs;
use remove::RemoveArgs;
use usev::UseArgs;

#[derive(Debug, Parser)]
#[clap(name = "solc-vm", about = "Solc version manager")]
enum SolcVm {
    #[clap(about = "List all versions of Solc")]
    List(ListArgs),
    #[clap(about = "Install Solc versions")]
    Install(InstallArgs),
    #[clap(about = "Use a Solc version")]
    Use(UseArgs), // { version: String },
    #[clap(about = "Remove a Solc version")]
    Remove(RemoveArgs), // { version: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = SolcVm::parse();

    svm_lib::setup_data_dir()?;

    match opt {
        SolcVm::List(cmd) => cmd.run().await?,
        SolcVm::Install(cmd) => cmd.run().await?,
        SolcVm::Use(cmd) => cmd.run().await?,
        SolcVm::Remove(cmd) => cmd.run().await?,
    }

    Ok(())
}
