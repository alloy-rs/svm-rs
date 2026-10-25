fn main() {
    #[cfg(feature = "cli")]
    {
        use smolgen::{Build, Emitter, Gitcl};
        use std::env;

        let mut instructions = Vec::new();
        Emitter::default()
            .default_on_error()
            .add_instructions(&Build::builder().build_date(true).build())
            .unwrap()
            .add_instructions(&Gitcl::builder().sha(true).build())
            .unwrap()
            .emit_to(&mut instructions)
            .unwrap();

        // Tracking the git HEAD reruns this script, and rebuilds this crate and everything above
        // it, on every commit and checkout. Debug builds skip it and keep the SHA from the last
        // run of this script. Release profiles keep tracking HEAD, so their version always
        // matches the checkout.
        let skip_head_tracking = env::var("PROFILE").is_ok_and(|profile| profile == "debug");
        for line in String::from_utf8(instructions).unwrap().lines() {
            let is_git_path = line.starts_with("cargo:rerun-if-changed=")
                && line != "cargo:rerun-if-changed=build.rs";
            if !(skip_head_tracking && is_git_path) {
                println!("{line}");
            }
        }
    }
}
