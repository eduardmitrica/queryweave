//! The feature checks of the target traits: each case builds with its
//! feature, and fails naming the feature without it. Cargo unifies features,
//! so a workspace build sees them all enabled; build this crate alone to check
//! the others (`cargo nextest run -p queryweave-sql --no-default-features`).

#[cfg(test)]
mod ui_tests {
    #[test]
    fn test_ui_features() {
        let t = trybuild::TestCases::new();

        let backends = [
            ("sqlx", cfg!(feature = "sqlx")),
            ("sea_query", cfg!(feature = "sea-query")),
            ("if_diesel", cfg!(feature = "diesel")),
        ];
        // rustc lists the trait's impls in the error, so the failing cases'
        // output depends on the other backends: check it without any.
        let without_backends = backends.iter().all(|(_, enabled)| !enabled);

        for (feature, enabled) in backends {
            let case = format!("tests/ui/features/{feature}.rs");
            if enabled {
                t.pass(case);
            } else if without_backends {
                t.compile_fail(case);
            }
        }

        // The merge-patch gate's failure names no backend: checked whenever
        // the feature is off.
        let case = "tests/ui/features/if_merge_patch.rs";
        if cfg!(feature = "merge-patch") {
            t.pass(case);
        } else {
            t.compile_fail(case);
        }
    }
}
