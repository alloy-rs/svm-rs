use semver::Version;
use structopt::StructOpt;

use std::collections::HashSet;

mod print;

#[derive(Debug, StructOpt)]
#[structopt(name = "solc-vm", about = "Solc version manager")]
enum SolcVm {
    #[structopt(about = "List all versions of Solc")]
    List,
    #[structopt(about = "Install a Solc version")]
    Install { version: String },
    #[structopt(about = "Use a Solc version")]
    Use { version: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = SolcVm::from_args();

    let all_versions = svm_lib::all_versions().await?;
    let installed_versions = svm_lib::installed_versions().unwrap_or_default();
    let current_version = svm_lib::current_version()?;

    match opt {
        SolcVm::List => {
            let a: HashSet<Version> = all_versions.iter().cloned().collect();
            let b: HashSet<Version> = installed_versions.iter().cloned().collect();
            let c = &a - &b;

            let mut available_versions = c.iter().cloned().collect::<Vec<Version>>();
            available_versions.sort();

            print::current_version(current_version);
            print::installed_versions(installed_versions);
            print::available_versions(available_versions);
        }
        SolcVm::Install { version } => {
            let version = Version::parse(&version)?;
            if installed_versions.contains(&version) {
                println!("Version: {:?} is already installed", version);
            } else if all_versions.contains(&version) {
                println!("Installing version: {:#?}", version);
                svm_lib::install(&version).await?;
                if current_version.is_none() {
                    svm_lib::use_version(&version)?;
                }
            } else {
                println!("Version: {:?} unsupported", version);
            }
        }
        SolcVm::Use { version } => {
            let version = Version::parse(&version)?;
            if installed_versions.contains(&version) {
                println!("Setting global version: {:?}", version);
                svm_lib::use_version(&version)?;
            } else if all_versions.contains(&version) {
                println!("Installed version: {:?}", version);
                svm_lib::install(&version).await?;
                println!("Setting global version: {:?}", version);
                svm_lib::use_version(&version)?;
            } else {
                println!("Version: {:?} unsupported", version);
            }
        }
    }

    Ok(())
}
