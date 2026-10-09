//! Checks on the names of the structs a derive is placed on or links to.
//!
//! The building blocks of [`crate::conventions::DefaultConventions`], which
//! names the generated types after these structs' pascal-case segments
//! (`OrderModel` → `Orders…Model`), so the names must split into enough of
//! them. Derives call them only through their conventions. Errors are
//! reported at the span given: the struct's name, or the path of the
//! attribute naming another struct.

use quote::ToTokens;

/// The struct named by `path` (e.g. of `#[merge_patch(patch = path)]`): its
/// last segment.
///
/// # Errors
///
/// Returns an error at the path when it has no segment, naming it `what`.
pub fn path_name<'a>(path: &'a syn::Path, what: &str) -> syn::Result<&'a syn::Ident> {
    let Some(segment) = path.segments.last() else {
        let message = format!("The path names no {what}");
        let help = format!("Name the {what} with a path, e.g. `crate::models::OrderModel`");
        return crate::errors::abort_spanned(path, &message, &help);
    };

    Ok(&segment.ident)
}

/// The pascal-case segments of the struct name `name`, which must have at
/// least `min_segments`; errors are reported at `span`, calling the struct
/// `what` (e.g. "struct", "merge patch model").
///
/// # Errors
///
/// Returns an error when `name` is not pascal cased, or has too few segments.
///
/// # Examples
///
/// ```rust
/// # use queryweave_codegen::names::pascal_case_segments;
/// let name: syn::Ident = syn::parse_quote!(OrderModel);
///
/// assert_eq!(
///     pascal_case_segments(&name, &name, "struct", 2).unwrap(),
///     vec!["Order", "Model"]
/// );
/// assert!(pascal_case_segments(&name, &name, "struct", 3).is_err());
/// ```
pub fn pascal_case_segments<S: ToTokens + ?Sized>(
    name: &syn::Ident,
    span: &S,
    what: &str,
    min_segments: usize,
) -> syn::Result<Vec<String>> {
    let name_str = name.to_string();
    let Ok(segments) = crate::utils::split_pascal_case(&name_str) else {
        let message = format!("The {what}'s name `{name}` is not pascal cased");
        let help = format!("Rename the {what} in PascalCase");
        return crate::errors::abort_spanned(span, &message, &help);
    };

    if segments.len() < min_segments {
        let message = format!(
            "The {what}'s name `{name}` must have at least {min_segments} pascal-case segments"
        );
        let help = format!(
            "Rename the {what} so its name has at least {min_segments} segments (`OrderModel` has two: `Order` and `Model`)"
        );
        return crate::errors::abort_spanned(span, &message, &help);
    }

    Ok(segments.into_iter().map(String::from).collect())
}

/// The pascal-case segments of the name of a merge patch struct: `MergePatch`
/// followed by at least two more (e.g. `MergePatchOrderModel`); errors are
/// reported at `span`, calling the struct `what`.
///
/// # Errors
///
/// Returns an error when `name` is not such a name.
///
/// # Examples
///
/// ```rust
/// # use queryweave_codegen::names::merge_patch_segments;
/// let name: syn::Ident = syn::parse_quote!(MergePatchOrderModel);
/// let misnamed: syn::Ident = syn::parse_quote!(PatchOrderModel);
///
/// assert_eq!(merge_patch_segments(&name, &name, "struct").unwrap().len(), 4);
/// assert!(merge_patch_segments(&misnamed, &misnamed, "struct").is_err());
/// ```
pub fn merge_patch_segments<S: ToTokens + ?Sized>(
    name: &syn::Ident,
    span: &S,
    what: &str,
) -> syn::Result<Vec<String>> {
    let segments = pascal_case_segments(name, span, what, 1)?;

    let starts_with_merge_patch = segments.first().is_some_and(|first| first == "Merge")
        && segments.get(1).is_some_and(|second| second == "Patch");
    if !starts_with_merge_patch || segments.len() < 4 {
        let message = format!(
            "The {what}'s name `{name}` must be `MergePatch` followed by at least two pascal-case segments"
        );
        let help = format!("Rename the {what}, e.g. `MergePatchOrderModel`");
        return crate::errors::abort_spanned(span, &message, &help);
    }

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::{merge_patch_segments, pascal_case_segments, path_name};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    fn ident(name: &str) -> syn::Ident {
        syn::parse_str(name).unwrap()
    }

    #[test]
    fn the_path_names_its_last_segment() {
        let path: syn::Path = syn::parse_quote!(crate::models::OrderModel);

        assert_eq!(path_name(&path, "model").unwrap(), "OrderModel");
    }

    #[rstest]
    #[case("OrderModel", 2, Some(vec!["Order", "Model"]))]
    #[case("OrderModel", 3, None)]
    #[case("Order", 2, None)]
    #[case("order_model", 1, None)]
    fn the_segments_are_counted_against_the_minimum(
        #[case] name: &str,
        #[case] min_segments: usize,
        #[case] expected: Option<Vec<&str>>,
    ) {
        let name = ident(name);

        assert_eq!(
            pascal_case_segments(&name, &name, "struct", min_segments).ok(),
            expected.map(|segments| segments.into_iter().map(String::from).collect())
        );
    }

    // Exactly four segments is enough: `MergePatch` + two.
    #[rstest]
    #[case("MergePatchOrderModel", true)]
    #[case("MergePatchOrderLineModel", true)]
    #[case("MergePatchModel", false)]
    #[case("PatchMergeOrderModel", false)]
    #[case("OrderMergePatchModel", false)]
    // Both leading segments are needed: `Merge` alone, or `Patch` alone.
    #[case("MergeOrderLineModel", false)]
    #[case("OrderPatchLineModel", false)]
    fn a_merge_patch_name_starts_with_merge_patch(#[case] name: &str, #[case] is_valid: bool) {
        let name = ident(name);

        assert_eq!(
            merge_patch_segments(&name, &name, "struct").is_ok(),
            is_valid
        );
    }

    // The error names the struct and the offending name.
    #[test]
    fn the_error_names_the_struct() {
        let name = ident("PatchOrderModel");
        let error = merge_patch_segments(&name, &name, "merge patch model").unwrap_err();

        assert!(error.to_string().starts_with(
            "The merge patch model's name `PatchOrderModel` must be `MergePatch` followed by at least two pascal-case segments"
        ));
    }
}
