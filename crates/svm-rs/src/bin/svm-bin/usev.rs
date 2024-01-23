use crate::print;
use clap::Parser;
use dialoguer::Input;
use semver::Version;

#[derive(Debug, Clone, Parser)]
pub struct UseArgs {
    /// Solc version to use
    pub version: String,
}

impl UseArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let version = Version::parse(&self.version)?;
        let all_versions = svm_lib::all_versions().await?;
        let installed_versions = svm_lib::installed_versions().unwrap_or_default();
        let current_version = svm_lib::current_version()?;

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
                let spinner = print::installing_version(&version);
                svm_lib::install(&version).await?;
                spinner.finish_with_message(format!("Downloaded Solc: {version}"));
                if current_version.is_none() {
                    svm_lib::use_version(&version)?;
                    print::set_global_version(&version);
                }
            }
        } else {
            print::unsupported_version(&version);
        }

        Ok(())
    }
}
