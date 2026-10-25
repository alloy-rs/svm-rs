fn main() {
    #[cfg(feature = "cli")]
    {
        use smolgen::{Build, Emitter, Gitcl};
        use std::env;

        // Debug builds skip git HEAD tracking, which would otherwise rerun this script, and
        // rebuild the crate, on every commit and checkout. Release profiles keep tracking HEAD.
        let track_head = env::var("PROFILE").unwrap() != "debug";
        Emitter::default()
            .default_on_error()
            .add_instructions(&Build::builder().build_date(true).build())
            .unwrap()
            .add_instructions(
                &Gitcl::builder()
                    .sha(true)
                    .rerun_on_head_change(track_head)
                    .build(),
            )
            .unwrap()
            .emit()
            .unwrap();
    }
}
