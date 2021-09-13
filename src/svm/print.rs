use crossterm::style::Stylize;
use itertools::Itertools;
use semver::Version;

pub fn current_version(version: Option<Version>) {
    match version {
        Some(v) => {
            println!("{} (current)", v.to_string().as_str().green());
        }
        None => {
            println!("Current version not set");
        }
    }
}

pub fn installed_versions(versions: Vec<Version>) {
    println!("\n{}", "Installed Versions".bold());
    versions.iter().for_each(|v| {
        println!("{}", v.to_string().as_str().yellow());
    });
}

pub fn available_versions(versions: Vec<Version>) {
    println!("\n{}", "Available to Install".bold());
    let groups = versions
        .iter()
        .group_by(|v| v.minor)
        .into_iter()
        .map(|(_, g)| g.cloned().collect())
        .collect::<Vec<Vec<Version>>>();
    for group in groups {
        println!(
            "{:?}",
            group.iter().map(|v| v.to_string()).collect::<Vec<String>>()
        );
    }
}
