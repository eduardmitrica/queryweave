//! Compile-fail and pass tests for the `queryweave` derives.

#[cfg(test)]
mod ui_tests {
    #[test]
    fn test_ui_filters() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/filters/fail/*.rs");
        t.pass("tests/ui/filters/pass/*.rs");
    }

    #[test]
    fn test_ui_merge_patch() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/merge_patch/fail/*.rs");
        t.pass("tests/ui/merge_patch/pass/*.rs");
    }

    #[test]
    fn test_ui_merge_patch_nullable_fields() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/merge_patch_nullable_fields/fail/*.rs");
        t.pass("tests/ui/merge_patch_nullable_fields/pass/*.rs");
    }

    #[test]
    fn test_ui_raw_value() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/raw_value/fail/*.rs");
        t.pass("tests/ui/raw_value/pass/*.rs");
    }

    #[test]
    fn test_ui_validate_non_nullable() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/validate_non_nullable/fail/*.rs");
        t.pass("tests/ui/validate_non_nullable/pass/*.rs");
    }

    #[test]
    fn test_ui_append_to_query() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/append_to_query/fail/*.rs");
        t.pass("tests/ui/append_to_query/pass/*.rs");
    }

    #[test]
    fn test_ui_append_conditions() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/append_conditions/fail/*.rs");
        t.pass("tests/ui/append_conditions/pass/*.rs");
    }

    #[test]
    fn test_ui_columns() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/columns/fail/*.rs");
        t.pass("tests/ui/columns/pass/*.rs");
    }

    #[test]
    fn test_ui_version() {
        let t = trybuild::TestCases::new();

        t.compile_fail("tests/ui/version/fail/*.rs");
    }
}
