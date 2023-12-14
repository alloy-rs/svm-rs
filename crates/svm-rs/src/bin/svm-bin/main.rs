//! Main svm-rs binary entry point.

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
        SolcVm::List(cmd) => utils::block_on(cmd.run())?,
        SolcVm::Install(cmd) => utils::block_on(cmd.run())?,
        SolcVm::Use(cmd) => utils::block_on(cmd.run())?,
        SolcVm::Remove(cmd) => utils::block_on(cmd.run())?,
    }

    Ok(())
}
