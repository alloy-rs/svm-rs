use crate::{install::InstallArgs, print};
use clap::Parser;
use dialoguer::Input;
use semver::Version;

#[derive(Debug, Clone, Parser)]
pub struct UseArgs {
    #[clap(long, short)]
    // TODO: Serde helper for parsing Version(?)
    pub version: String,
}

impl UseArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let version = Version::parse(&self.version)?;
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
                // TODO: Break up install fn into minimal install
                crate::install::InstallArgs::run(InstallArgs {
                    versions: vec![self.version],
                })
                .await?;
            }
        } else {
            print::unsupported_version(&version);
        }

        Ok(())
    }
}
