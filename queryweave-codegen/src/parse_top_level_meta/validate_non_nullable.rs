/// The struct attributes of a `ValidateNonNullable` merge patch DTO, whose
/// integrations are the conventions `C`'s.
pub struct TopLevelMeta<C> {
    /// `#[validate_non_nullable(mirrors = path)]`: the domain merge patch
    /// model the DTO mirrors.
    pub mirrors: syn::Path,
    /// `#[validate_non_nullable(validator)]` (or `garde`, ...): the
    /// integration defining `validate_non_nullable`, if any.
    pub integrations: crate::parse_top_level_meta::filters::Integrations,
    /// `#[queryweave(crate = path)]`: where the runtime crate the generated
    /// code names lives, overriding the conventions' default.
    pub runtime_crate: Option<syn::Path>,
    /// The conventions whose integrations the flags name.
    conventions: std::marker::PhantomData<fn() -> C>,
}

/// Read `#[validate_non_nullable(...)]`: `mirrors`, and a flag per
/// integration of `registry`, whose order the flags keep.
fn read(
    attributes: &[syn::Attribute],
    registry: &[&'static dyn crate::integrations::Integration],
) -> syn::Result<(
    syn::Path,
    crate::parse_top_level_meta::filters::Integrations,
)> {
    crate::integrations::check_registry(registry)?;
    let options = std::iter::once(("mirrors", crate::attributes::OptionKind::Path))
        .chain(
            registry
                .iter()
                .map(|integration| (integration.name(), crate::attributes::OptionKind::Flag)),
        )
        .collect::<Vec<_>>();

    let mirrors = crate::attributes::required_path_option(
        attributes,
        "validate_non_nullable",
        "mirrors",
        &options,
        "Name the domain merge patch model this DTO mirrors: #[validate_non_nullable(mirrors = path)]",
    )?;
    let mut flags = Vec::new();
    for integration in registry {
        flags.extend(crate::attributes::flag_option(
            attributes,
            "validate_non_nullable",
            integration.name(),
            &options,
        )?);
    }

    Ok((
        mirrors,
        crate::parse_top_level_meta::filters::Integrations::from_flags(flags),
    ))
}

impl<C: crate::conventions::Conventions> crate::steps::TopLevelMeta for TopLevelMeta<C> {
    fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self> {
        let validate_non_nullable = read(attributes, &C::integrations());
        let runtime_crate = crate::attributes::crate_option(attributes);

        // Report every malformed attribute at once.
        let ((mirrors, integrations), runtime_crate) =
            crate::errors::both(validate_non_nullable, runtime_crate)?;

        Ok(Self {
            mirrors,
            integrations,
            runtime_crate,
            conventions: std::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {

    /// The meta read from `item`'s attributes, under the default conventions:
    /// the mirrored model and the flags.
    fn parse(item: proc_macro2::TokenStream) -> syn::Result<(syn::Path, Vec<syn::Ident>)> {
        let attributes = syn::parse2::<syn::DeriveInput>(item).unwrap().attrs;
        let meta: super::TopLevelMeta<crate::conventions::DefaultConventions> =
            crate::steps::TopLevelMeta::from_attributes(&attributes)?;

        Ok((meta.mirrors, meta.integrations.flags().to_vec()))
    }

    #[test]
    fn test_parsing_top_level_meta_succeeds_when_all_preconditions_are_met() {
        let top_level_meta = parse(quote::quote! {
            #[derive(ValidateNonNullable, Validate)]
            #[validate_non_nullable(mirrors = MergePatchOrderModel)]
            pub struct MergePatchOrderDto {
                pub value: Option<Option<f64>>,
                pub delivery_date: Option<Option<DateTime<Utc>>>,
            }
        })
        .unwrap();

        pretty_assertions::assert_eq!(
            top_level_meta,
            (syn::parse_str("MergePatchOrderModel").unwrap(), Vec::new())
        );
    }

    // The integration defining `validate_non_nullable` is a flag.
    #[test]
    fn test_parsing_top_level_meta_reads_the_flags() {
        let (_, flags) = parse(quote::quote! {
            #[validate_non_nullable(validator, mirrors = MergePatchOrderModel)]
            pub struct MergePatchOrderDto;
        })
        .unwrap();

        pretty_assertions::assert_eq!(flags, [quote::format_ident!("validator")]);
    }

    #[test]
    fn test_parsing_top_level_meta_fails_without_the_mirrored_model() {
        assert!(
            parse(quote::quote! {
                pub struct MergePatchOrderDto {
                    pub value: Option<Option<f64>>,
                }
            })
            .is_err()
        );
    }

    #[test]
    fn test_parsing_top_level_meta_fails_for_a_malformed_mirrors_option() {
        for item in [
            quote::quote! { #[validate_non_nullable()] pub struct MergePatchOrderDto; },
            quote::quote! { #[validate_non_nullable(mirrors)] pub struct MergePatchOrderDto; },
            quote::quote! {
                #[validate_non_nullable(mirrors = domain::models::merge_patch_order::)]
                pub struct MergePatchOrderDto;
            },
            quote::quote! {
                #[validate_non_nullable(mirrors = MergePatchOrderModel)]
                #[validate_non_nullable(mirrors = OtherMergePatchModel)]
                pub struct MergePatchOrderDto;
            },
            quote::quote! { #[validate_non_nullable(mirror = MergePatchOrderModel)] pub struct MergePatchOrderDto; },
        ] {
            assert!(parse(item).is_err());
        }
    }
}
