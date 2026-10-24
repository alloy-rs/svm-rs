fn main() {
    #[cfg(feature = "cli")]
    {
        use smolgen::{Build, Emitter, Gitcl};

        Emitter::default()
            .default_on_error()
            .add_instructions(&Build::builder().build_date(true).build())
            .unwrap()
            .add_instructions(&Gitcl::builder().sha(true).build())
            .unwrap()
            .emit()
            .unwrap();
    }
}
