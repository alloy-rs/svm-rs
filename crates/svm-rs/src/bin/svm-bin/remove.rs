use crate::print;
use clap::Parser;
use dialoguer::Input;
use semver::Version;

/// Remove a Solc version, or "all" to remove all versions.
#[derive(Clone, Debug, Parser)]
pub struct RemoveCmd {
    /// Solc version to remove, or "all" to remove all versions.
    pub version: String,
}

impl RemoveCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        if self.version.to_ascii_lowercase() == "all" {
            for v in svm::installed_versions().unwrap_or_default() {
                svm::remove_version(&v)?;
            }
            svm::unset_global_version()?;
            return Ok(());
        } else {
            let mut installed_versions = svm::installed_versions().unwrap_or_default();
            let current_version = svm::get_global_version()?;
            let version = Version::parse(&self.version)?;

            if installed_versions.contains(&version) {
                let input: String = Input::new()
                    .with_prompt("Are you sure?")
                    .with_initial_text("Y")
                    .default("N".into())
                    .interact_text()?;
                if matches!(input.as_str(), "y" | "Y" | "yes" | "Yes") {
                    svm::remove_version(&version)?;
                    if let Some(v) = current_version {
                        if version == v {
                            if let Some(i) = installed_versions.iter().position(|x| *x == v) {
                                installed_versions.remove(i);
                                if let Some(new_version) = installed_versions.pop() {
                                    svm::set_global_version(&new_version)?;
                                    print::set_global_version(&new_version);
                                } else {
                                    svm::unset_global_version()?;
                                }
                            }
                        }
                    }
                }
            } else {
                print::version_not_found(&version);
            }
        }

        Ok(())
    }
}
