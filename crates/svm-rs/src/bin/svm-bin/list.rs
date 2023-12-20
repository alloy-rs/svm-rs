use std::collections::HashSet;

use crate::print;
use clap::Parser;
use semver::Version;

#[derive(Debug, Parser)]
pub struct ListArgs;

impl ListArgs {
    pub async fn run(self) -> anyhow::Result<()> {
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
}
