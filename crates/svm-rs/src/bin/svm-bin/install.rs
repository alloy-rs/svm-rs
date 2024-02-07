use crate::print;
use clap::Parser;
use dialoguer::Input;
use semver::Version;

/// Install Solc versions.
#[derive(Clone, Debug, PartialEq, Eq, Parser)]
pub struct InstallCmd {
    /// Solc versions to install.
    pub versions: Vec<String>,
}

impl InstallCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let all_versions = svm::all_versions().await?;

        for version in self.versions {
            let installed_versions = svm::installed_versions().unwrap_or_default();
            let current_version = svm::get_global_version()?;
            let version = Version::parse(&version)?;

            if installed_versions.contains(&version) {
                println!("Solc {version} is already installed");
                let input: String = Input::new()
                    .with_prompt("Would you like to set it as the global version?")
                    .with_initial_text("Y")
                    .default("N".into())
                    .interact_text()?;
                if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
                    svm::set_global_version(&version)?;
                    print::set_global_version(&version);
                }
            } else if all_versions.contains(&version) {
                let spinner = print::installing_version(&version);
                svm::install(&version).await?;
                spinner.finish_with_message(format!("Downloaded Solc: {version}"));
                if current_version.is_none() {
                    svm::set_global_version(&version)?;
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
        let args: InstallCmd = InstallCmd::parse_from(["svm", "0.8.11", "0.8.10"]);
        assert_eq!(
            args,
            InstallCmd {
                versions: vec!["0.8.11".into(), "0.8.10".into()]
            }
        );
    }
}
