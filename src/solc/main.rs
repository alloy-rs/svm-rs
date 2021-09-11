use std::{env, process::Command};

fn main() -> anyhow::Result<()> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    let version = solc_vm_lib::current_version()?;
    let mut version_path = solc_vm_lib::version_path(&solc_vm_lib::home(), &version);
    version_path.push(format!("solc-{}", &version));

    Command::new(version_path).args(args).spawn()?;

    Ok(())
}
