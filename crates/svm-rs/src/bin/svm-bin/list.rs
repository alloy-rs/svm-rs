use std::collections::HashSet;

use crate::print;
use clap::Parser;
use semver::Version;

/// List all Solc versions.
#[derive(Debug, Parser)]
pub struct ListCmd;

impl ListCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let mut failed = false;
        let mut err = |e: &svm::SvmError, s: &str| {
            failed = true;
            eprintln!("{s}: {e}");
        };

        let all_versions = svm::all_versions()
            .await
            .inspect_err(|e| err(e, "Error fetching all versions"))
            .unwrap_or_default();
        let installed_versions = svm::installed_versions()
            .inspect_err(|e| err(e, "Error fetching installed versions"))
            .unwrap_or_default();
        let current_version =
            svm::get_global_version().inspect_err(|e| err(e, "Error fetching current version"));

        let mut available_versions = {
            let a: HashSet<Version> = all_versions.iter().cloned().collect();
            let b: HashSet<Version> = installed_versions.iter().cloned().collect();
            let c = &a - &b;
            c.iter().cloned().collect::<Vec<Version>>()
        };
        available_versions.sort();

        if let Ok(current_version) = current_version {
            print::current_version(current_version);
        }
        if !installed_versions.is_empty() {
            print::installed_versions(installed_versions);
        }
        if !available_versions.is_empty() {
            print::available_versions(available_versions);
        }

        if failed {
            std::process::exit(1);
        }

        Ok(())
    }
}
