use clap::Parser;
use semver::Version;

/// Display which binary will be run for a given version.
#[derive(Debug, Parser)]
pub struct WhichCmd {
    /// The version to check.
    version: Version,
}

impl WhichCmd {
    pub fn run(self) -> anyhow::Result<()> {
        let Self { version } = self;
        let bin = svm::version_binary(&version.to_string());
        if bin.exists() {
            println!("{}", bin.display());
        } else {
            return Err(anyhow::anyhow!("version {version} not installed"));
        }
        Ok(())
    }
}
