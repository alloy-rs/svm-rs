use std::{env, process::Command};

fn main() -> anyhow::Result<()> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    let version = svm_lib::current_version()?;
    let mut version_path = svm_lib::version_path(&version);
    version_path.push(format!("solc-{}", &version));

    Command::new(version_path).args(args).spawn()?;

    Ok(())
}
