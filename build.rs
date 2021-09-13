use std::fs::{self, File};

fn main() {
    // Create the SVM home directory.
    let mut svm_home = home::home_dir().expect("could not detect user home directory");
    svm_home.push(".svm");
    fs::create_dir_all(svm_home.clone()).expect("could not create SVM home directory");

    // Create the SVM global version file.
    let mut global_version = svm_home;
    global_version.push(".global-version");
    File::create(global_version.as_path()).expect("could not create SVM global version file");

    println!("cargo::rerun-if-changed=build.rs");
}
