#[derive(Debug, PartialEq, Eq)]
pub struct MappingPaths {
    pub equivalent: syn::Path,
    /// The path of the runtime crate the generated code names: the one
    /// given by `#[queryweave(crate = path)]`, or the conventions' core (the
    /// generated code names only core items).
    pub runtime: syn::Path,
    /// The integration defining `validate_non_nullable`, if one is written.
    pub integration: Option<crate::integrations::Active>,
}

/// Check the domain merge patch model named by
/// `#[validate_non_nullable(mirrors = path)]`, and the integration written.
///
/// The path is used as written: nothing about it is derived from the DTO's
/// own name.
///
/// # Errors
///
/// Returns an error, at the path, when it names no struct, the struct `ident`
/// itself, or one whose name is not a merge patch one under the conventions
/// `C`; and at a flag naming an integration that does not apply to
/// `ValidateNonNullable`, or a second one that does.
pub fn generate_paths<C: crate::conventions::Conventions>(
    ident: &syn::Ident,
    path: syn::Path,
    integrations: &crate::parse_top_level_meta::filters::Integrations,
    runtime_crate: Option<syn::Path>,
) -> syn::Result<MappingPaths> {
    crate::attributes::reject_self_link(&path, ident, "validate_non_nullable", "mirrors")?;
    let what = "domain merge patch model";
    C::check_merge_patch_name(crate::names::path_name(&path, what)?, &path, what)?;

    let registry = C::integrations();
    let supported = registry
        .iter()
        .filter(|integration| integration.supports_validate_non_nullable())
        .map(|integration| integration.name())
        .collect::<Vec<_>>()
        .join(", ");
    let mut errors = crate::errors::Errors::default();
    let mut integration: Option<crate::integrations::Active> = None;
    for active in crate::mapping::filters::resolve(&registry, integrations) {
        let flag = &active.flag;
        if !active.integration.supports_validate_non_nullable() {
            errors.push(syn::Error::new(
                flag.span(),
                crate::errors::message(
                    &format!("`{flag}` does not apply to ValidateNonNullable"),
                    &format!("Name the validation library defining validate_non_nullable: one of {supported}"),
                ),
            ));
        } else if let Some(first) = &integration {
            errors.push(syn::Error::new(
                flag.span(),
                crate::errors::message(
                    &format!(
                        "`{}` and `{flag}` would both define validate_non_nullable",
                        first.flag
                    ),
                    "Keep the one validation library the DTO uses",
                ),
            ));
        } else {
            integration = Some(active);
        }
    }

    errors.finish(MappingPaths {
        equivalent: path,
        runtime: runtime_crate.unwrap_or_else(C::core_crate),
        integration,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_generating_mapping_paths_succeeds_when_all_preconditions_are_met() {
        let mapping_paths = super::generate_paths::<crate::conventions::DefaultConventions>(
            &quote::format_ident!("MergePatchOrderDto"),
            syn::parse_str("domain::models::merge_patch_order::MergePatchOrderModel").unwrap(),
            &crate::parse_top_level_meta::filters::Integrations::default(),
            None,
        )
        .unwrap();

        pretty_assertions::assert_eq!(
            mapping_paths,
            super::MappingPaths {
                equivalent: syn::parse_str(
                    "domain::models::merge_patch_order::MergePatchOrderModel"
                )
                .unwrap(),
                runtime: syn::parse_quote!(::queryweave),
                integration: None,
            }
        );
    }

    #[test]
    fn test_generating_mapping_paths_fails_for_a_name_that_is_not_a_merge_patch_one() {
        // Not pascal cased, then not a merge patch name.
        for path in ["merge_patch_order_model", "OrderModel"] {
            assert!(
                super::generate_paths::<crate::conventions::DefaultConventions>(
                    &quote::format_ident!("MergePatchOrderDto"),
                    syn::parse_str(path).unwrap(),
                    &crate::parse_top_level_meta::filters::Integrations::default(),
                    None
                )
                .is_err()
            );
        }
    }

    // A struct mirroring itself, by `Self` or by its own name, never reaches a
    // merge patch model; a same-named struct of another module may be meant.
    #[test]
    fn test_generating_mapping_paths_rejects_a_struct_mirroring_itself() {
        let generate = |path: &str| {
            super::generate_paths::<crate::conventions::DefaultConventions>(
                &quote::format_ident!("MergePatchOrderDto"),
                syn::parse_str(path).unwrap(),
                &crate::parse_top_level_meta::filters::Integrations::default(),
                None,
            )
        };

        assert!(generate("Self").is_err());
        assert!(generate("MergePatchOrderDto").is_err());
        assert!(generate("self::MergePatchOrderDto").is_err());
        assert!(generate("api::MergePatchOrderDto").is_ok());
    }

    /// `generate_paths` for `MergePatchOrderDto` with the flags `names`.
    fn with_flags(names: &[&str]) -> syn::Result<super::MappingPaths> {
        super::generate_paths::<crate::conventions::DefaultConventions>(
            &quote::format_ident!("MergePatchOrderDto"),
            syn::parse_quote!(MergePatchOrderModel),
            &crate::parse_top_level_meta::filters::Integrations::of(names),
            None,
        )
    }

    // One validation library defines `validate_non_nullable`; none leaves
    // only `rejected_nulls`.
    #[test]
    fn test_one_validation_integration_is_kept() {
        assert!(with_flags(&[]).unwrap().integration.is_none());
        let integration = with_flags(&["garde"]).unwrap().integration.unwrap();
        assert_eq!(integration.flag, "garde");
    }

    // A flag that is no validation library, or a second one, is an error.
    #[test]
    fn test_other_or_several_integrations_are_rejected() {
        assert!(with_flags(&["serde"]).is_err());
        assert!(with_flags(&["validator", "garde"]).is_err());
    }
}
