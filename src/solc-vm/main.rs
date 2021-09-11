use std::collections::HashSet;
use structopt::StructOpt;

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

    let all_versions = solc_vm_lib::all_versions().await?;
    let installed_versions = solc_vm_lib::installed_versions().unwrap_or_default();
    let current_version = solc_vm_lib::current_version().unwrap_or_else(|_| "".to_string());

    match opt {
        SolcVm::List => {
            let a: HashSet<String> = all_versions.iter().cloned().collect();
            let b: HashSet<String> = installed_versions.iter().cloned().collect();
            let c = &a - &b;
            println!("Current version: {:?}", current_version);
            println!("Installed versions");
            println!("{:#?}", installed_versions);
            println!("Available versions");
            println!("{:#?}", c.iter().cloned().collect::<Vec<String>>());
        }
        SolcVm::Install { version } => {
            if installed_versions.contains(&version) {
                println!("Version: {:?} is already installed", version);
            } else if all_versions.contains(&version) {
                println!("Installing version: {:#?}", version);
                solc_vm_lib::install(&version).await?;
            } else {
                println!("Version: {:?} unsupported", version);
            }
        }
        SolcVm::Use { version } => {
            if installed_versions.contains(&version) {
                println!("Setting global version: {:?}", version);
                solc_vm_lib::use_version(&version)?;
            } else if all_versions.contains(&version) {
                println!("Installed version: {:?}", version);
                solc_vm_lib::install(&version).await?;
                println!("Setting global version: {:?}", version);
                solc_vm_lib::use_version(&version)?;
            } else {
                println!("Version: {:?} unsupported", version);
            }
        }
    }

    Ok(())
}
