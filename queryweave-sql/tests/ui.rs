//! The feature checks of the target traits: each case builds with its
//! feature, and fails naming the feature without it. Cargo unifies features,
//! so a workspace build sees them all enabled; build this crate alone to check
//! the others (`cargo nextest run -p queryweave-sql --no-default-features`).

#[cfg(test)]
mod ui_tests {
    #[test]
    fn test_ui_features() {
        let t = trybuild::TestCases::new();

        let case = "tests/ui/features/sqlx.rs";
        if cfg!(feature = "sqlx") {
            t.pass(case);
        } else {
            t.compile_fail(case);
        }
    }
}
