//! The feature checks of `features`: each case builds with its feature, and
//! fails naming the feature without it. Cargo unifies features, so a
//! workspace build sees them all enabled; build this crate alone to check
//! the others (`cargo nextest run -p queryweave-api --features ...`).

#[cfg(test)]
mod ui_tests {
    #[test]
    fn test_ui_features() {
        let t = trybuild::TestCases::new();

        for (feature, enabled) in [
            ("serde", cfg!(feature = "serde")),
            ("utoipa", cfg!(feature = "utoipa")),
            ("schemars", cfg!(feature = "schemars")),
        ] {
            let case = format!("tests/ui/features/{feature}.rs");
            if enabled {
                t.pass(case);
            } else {
                t.compile_fail(case);
            }
        }
    }
}
