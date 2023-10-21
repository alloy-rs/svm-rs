use clap::Parser;
use dialoguer::Input;
use semver::Version;

use std::collections::HashSet;

mod print;

#[derive(Debug, Parser)]
#[clap(name = "solc-vm", about = "Solc version manager")]
enum SolcVm {
    #[clap(about = "List all versions of Solc")]
    List,
    #[clap(about = "Install Solc versions")]
    Install { versions: Vec<String> },
    #[clap(about = "Use a Solc version")]
    Use { version: String },
    #[clap(about = "Remove a Solc version")]
    Remove { version: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = SolcVm::parse();

    svm_lib::setup_data_dir()?;

    match opt {
        SolcVm::List => {
            handle_list().await?;
        }
        SolcVm::Install { versions } => {
            for v in versions {
                handle_install(Version::parse(&v)?).await?;
            }
        }
        SolcVm::Use { version } => {
            handle_use(Version::parse(&version)?).await?;
        }
        SolcVm::Remove { version } => match version.as_str() {
            "ALL" | "all" => {
                for v in svm_lib::installed_versions().unwrap_or_default() {
                    svm_lib::remove_version(&v)?;
                }
                svm_lib::unset_global_version()?;
            }
            _ => handle_remove(Version::parse(&version)?)?,
        },
    }

    Ok(())
}

async fn handle_list() -> anyhow::Result<()> {
    let all_versions = svm_lib::all_versions().await?;
    let installed_versions = svm_lib::installed_versions().unwrap_or_default();
    let current_version = svm_lib::current_version()?;

    let a: HashSet<Version> = all_versions.iter().cloned().collect();
    let b: HashSet<Version> = installed_versions.iter().cloned().collect();
    let c = &a - &b;

    let mut available_versions = c.iter().cloned().collect::<Vec<Version>>();
    available_versions.sort();

    print::current_version(current_version);
    print::installed_versions(installed_versions);
    print::available_versions(available_versions);

    Ok(())
}

async fn handle_install(version: Version) -> anyhow::Result<()> {
    let all_versions = svm_lib::all_versions().await?;
    let installed_versions = svm_lib::installed_versions().unwrap_or_default();
    let current_version = svm_lib::current_version()?;

    if installed_versions.contains(&version) {
        println!("Solc {version} is already installed");
        let input: String = Input::new()
            .with_prompt("Would you like to set it as the global version?")
            .with_initial_text("Y")
            .default("N".into())
            .interact_text()?;
        if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
            svm_lib::use_version(&version)?;
            print::set_global_version(&version);
        }
    } else if all_versions.contains(&version) {
        let spinner = print::installing_version(&version);
        svm_lib::install(&version).await?;
        spinner.finish_with_message(format!("Downloaded Solc: {version}"));
        if current_version.is_none() {
            svm_lib::use_version(&version)?;
            print::set_global_version(&version);
        }
    } else {
        print::unsupported_version(&version);
    }

    Ok(())
}

async fn handle_use(version: Version) -> anyhow::Result<()> {
    let all_versions = svm_lib::all_versions().await?;
    let installed_versions = svm_lib::installed_versions().unwrap_or_default();

    if installed_versions.contains(&version) {
        svm_lib::use_version(&version)?;
        print::set_global_version(&version);
    } else if all_versions.contains(&version) {
        println!("Solc {version} is not installed");
        let input: String = Input::new()
            .with_prompt("Would you like to install it?")
            .with_initial_text("Y")
            .default("N".into())
            .interact_text()?;
        if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
            handle_install(version).await?;
        }
    } else {
        print::unsupported_version(&version);
    }

    Ok(())
}

fn handle_remove(version: Version) -> anyhow::Result<()> {
    let mut installed_versions = svm_lib::installed_versions().unwrap_or_default();
    let current_version = svm_lib::current_version()?;

    if installed_versions.contains(&version) {
        let input: String = Input::new()
            .with_prompt("Are you sure?")
            .with_initial_text("Y")
            .default("N".into())
            .interact_text()?;
        if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
            svm_lib::remove_version(&version)?;
            if let Some(v) = current_version {
                if version == v {
                    if let Some(i) = installed_versions.iter().position(|x| *x == v) {
                        installed_versions.remove(i);
                        if let Some(new_version) = installed_versions.pop() {
                            svm_lib::use_version(&new_version)?;
                            print::set_global_version(&new_version);
                        } else {
                            svm_lib::unset_global_version()?;
                        }
                    }
                }
            }
        }
    } else {
        print::version_not_found(&version);
    }

    Ok(())
}
