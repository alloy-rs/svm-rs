use crate::print;
use clap::Parser;
use dialoguer::Input;
use semver::Version;

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
pub struct InstallArgs {
    /// Solc versions to install
    pub versions: Vec<String>,
}

impl InstallArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        let all_versions = svm_lib::all_versions().await?;

        for version in self.versions {
            let installed_versions = svm_lib::installed_versions().unwrap_or_default();
            let current_version = svm_lib::current_version()?;
            let version = Version::parse(&version)?;

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
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_install() {
        let args: InstallArgs = InstallArgs::parse_from(["svm", "0.8.11", "0.8.10"]);
        assert_eq!(
            args,
            InstallArgs {
                versions: vec!["0.8.11".into(), "0.8.10".into()]
            }
        );
    }
}
