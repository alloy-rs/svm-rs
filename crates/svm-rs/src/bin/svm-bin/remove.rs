use crate::print;
use clap::Parser;
use dialoguer::Input;
use semver::Version;

#[derive(Debug, Clone, Parser)]
pub struct RemoveArgs {
    /// Solc version to remove
    pub version: String,
}

impl RemoveArgs {
    pub async fn run(self) -> anyhow::Result<()> {
        if self.version.to_ascii_lowercase() == "all" {
            for v in svm_lib::installed_versions().unwrap_or_default() {
                svm_lib::remove_version(&v)?;
            }
            svm_lib::unset_global_version()?;
            return Ok(());
        } else {
            let mut installed_versions = svm_lib::installed_versions().unwrap_or_default();
            let current_version = svm_lib::current_version()?;
            let version = Version::parse(&self.version)?;

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
        }

        Ok(())
    }
}
