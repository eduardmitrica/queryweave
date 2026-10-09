#[derive(Debug, PartialEq, Eq)]
pub struct MappingPaths {
    pub equivalent: syn::Path,
    /// The path of the runtime crate the generated code names: the one
    /// given by `#[queryweave(crate = path)]`, or the conventions' default.
    pub runtime: syn::Path,
}

/// Check the merge patch model named by `#[merge_patch(patch = path)]`.
///
/// # Errors
///
/// Returns an error, at the path, when it names no struct, or one whose name
/// is not a merge patch one under the conventions `C`.
pub fn generate_paths<C: crate::conventions::Conventions>(
    path: syn::Path,
    runtime_crate: Option<syn::Path>,
) -> syn::Result<MappingPaths> {
    let what = "merge patch model";
    C::check_merge_patch_name(crate::names::path_name(&path, what)?, &path, what)?;

    Ok(MappingPaths {
        equivalent: path,
        runtime: runtime_crate.unwrap_or_else(C::core_crate),
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    #[test]
    fn test_generating_mapping_paths_succeeds_when_all_preconditions_are_met() {
        let mapping_paths = super::generate_paths::<crate::conventions::DefaultConventions>(
            syn::parse_str("crate::models::MergePatchOrderModel").unwrap(),
            None,
        )
        .unwrap();

        pretty_assertions::assert_eq!(
            mapping_paths,
            super::MappingPaths {
                equivalent: syn::parse_str("crate::models::MergePatchOrderModel").unwrap(),
                runtime: syn::parse_quote!(::queryweave),
            }
        );
    }

    #[rstest]
    // Not pascal cased.
    #[case("merge_patch_order_model")]
    // Not a merge patch name, or too short to be one.
    #[case("OrderModel")]
    #[case("PatchOrderModel")]
    #[case("MergePatchModel")]
    fn test_generating_mapping_paths_fails_for_a_name_that_is_not_a_merge_patch_one(
        #[case] path: &str,
    ) {
        assert!(
            super::generate_paths::<crate::conventions::DefaultConventions>(
                syn::parse_str(path).unwrap(),
                None
            )
            .is_err()
        );
    }

    /// Conventions naming merge patch structs `…Patch`.
    struct PatchSuffixConventions;

    impl crate::conventions::Conventions for PatchSuffixConventions {
        fn filters_names(input: &syn::Ident) -> syn::Result<crate::conventions::FiltersNames> {
            crate::conventions::DefaultConventions::filters_names(input)
        }

        fn check_model_name(_name: &syn::Ident) -> syn::Result<()> {
            Ok(())
        }

        fn check_merge_patch_name<S: quote::ToTokens + ?Sized>(
            name: &syn::Ident,
            span: &S,
            what: &str,
        ) -> syn::Result<()> {
            if name.to_string().ends_with("Patch") {
                Ok(())
            } else {
                crate::errors::abort_spanned(
                    span,
                    &format!("The {what}'s name must end in `Patch`"),
                    "Rename it",
                )
            }
        }

        fn sort_variant(field: &syn::Ident, span: proc_macro2::Span) -> syn::Ident {
            crate::conventions::DefaultConventions::sort_variant(field, span)
        }

        fn filter_field(field: &syn::Ident, operator: &str, span: proc_macro2::Span) -> syn::Ident {
            crate::conventions::DefaultConventions::filter_field(field, operator, span)
        }

        fn check_wrapper_name<S: quote::ToTokens + ?Sized>(
            name: &syn::Ident,
            span: &S,
            what: &str,
        ) -> syn::Result<()> {
            crate::conventions::DefaultConventions::check_wrapper_name(name, span, what)
        }
    }

    // The merge patch model's name is checked by the derive's conventions.
    #[test]
    fn test_generating_mapping_paths_checks_the_name_against_the_conventions() {
        let path = || syn::parse_str::<syn::Path>("crate::models::OrderPatch").unwrap();

        assert!(super::generate_paths::<PatchSuffixConventions>(path(), None).is_ok());
        assert!(
            super::generate_paths::<crate::conventions::DefaultConventions>(path(), None).is_err()
        );
        assert!(
            super::generate_paths::<PatchSuffixConventions>(
                syn::parse_str("crate::models::MergePatchOrderModel").unwrap(),
                None,
            )
            .is_err()
        );
    }
}
