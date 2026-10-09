/// The options of `#[merge_patch(...)]` on the struct (the field attribute of
/// the same name, `#[merge_patch(skip)]`, is read by the parse step).
const MERGE_PATCH_OPTIONS: [(&str, crate::attributes::OptionKind); 1] =
    [("patch", crate::attributes::OptionKind::Path)];

/// The struct attributes of a `MergePatch` model.
#[derive(Debug, PartialEq, Eq)]
pub struct TopLevelMeta {
    /// `#[merge_patch(patch = path)]`: the merge patch model merged into the
    /// model.
    pub merge_patch_model: syn::Path,
    /// `#[queryweave(crate = path)]`: where the runtime crate the generated
    /// code names lives, overriding the conventions' default.
    pub runtime_crate: Option<syn::Path>,
}

impl crate::steps::TopLevelMeta for TopLevelMeta {
    fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self> {
        let merge_patch_model = crate::attributes::required_path_option(
            attributes,
            "merge_patch",
            "patch",
            &MERGE_PATCH_OPTIONS,
            "Name the merge patch model merged into this model: #[merge_patch(patch = path)]",
        );
        let runtime_crate = crate::attributes::crate_option(attributes);

        // Report every malformed attribute at once.
        let (merge_patch_model, runtime_crate) =
            crate::errors::both(merge_patch_model, runtime_crate)?;

        Ok(Self {
            merge_patch_model,
            runtime_crate,
        })
    }
}

#[cfg(test)]
mod tests {

    fn parse(item: proc_macro2::TokenStream) -> syn::Result<super::TopLevelMeta> {
        let attributes = syn::parse2::<syn::DeriveInput>(item).unwrap().attrs;

        crate::steps::TopLevelMeta::from_attributes(&attributes)
    }

    #[test]
    fn test_parsing_top_level_meta_succeeds_when_all_preconditions_are_met() {
        let top_level_meta = parse(quote::quote! {
            #[derive(MergePatch)]
            #[merge_patch(patch = MergePatchOrderModel)]
            struct OrderModel {
                client: Option<String>,

                #[merge_patch(skip)]
                created_by_user_with_id: u32,
            }
        })
        .unwrap();

        pretty_assertions::assert_eq!(
            top_level_meta,
            super::TopLevelMeta {
                merge_patch_model: syn::parse_str("MergePatchOrderModel").unwrap(),
                runtime_crate: None,
            }
        );
    }

    // An empty attribute, or `patch` without its path.
    #[test]
    fn test_parsing_top_level_meta_fails_when_empty_path_is_provided() {
        for item in [
            quote::quote! { #[merge_patch()] struct OrderModel { client: Option<String> } },
            quote::quote! { #[merge_patch(patch)] struct OrderModel { client: Option<String> } },
        ] {
            assert!(parse(item).is_err());
        }
    }

    // The model's own `#[merge_patch(...)]` options are checked; a field's
    // (`#[merge_patch(skip)]`) are the parse step's.
    #[test]
    fn test_parsing_top_level_meta_fails_for_an_unknown_option() {
        assert!(
            parse(quote::quote! {
                #[merge_patch(patch = MergePatchOrderModel, skip)]
                struct OrderModel {
                    client: Option<String>,
                }
            })
            .is_err()
        );
    }

    #[test]
    fn test_parsing_top_level_meta_fails_when_no_last_path_segment_is_provided() {
        assert!(
            parse(quote::quote! {
                #[merge_patch(patch = crate::merge_patch_order::)]
                struct OrderModel {
                    client: Option<String>,
                }
            })
            .is_err()
        );
    }

    // A second model, in another attribute or the same one, is rejected
    // rather than silently ignored.
    #[test]
    fn test_parsing_top_level_meta_fails_when_more_than_one_model_is_provided() {
        assert!(
            parse(quote::quote! {
                #[merge_patch(patch = MergePatchOrderModel)]
                #[merge_patch(patch = OtherMergePatchModel)]
                struct OrderModel {
                    client: Option<String>,
                }
            })
            .is_err()
        );
        assert!(
            parse(quote::quote! {
                #[merge_patch(patch = MergePatchOrderModel, OtherMergePatchModel)]
                struct OrderModel {
                    client: Option<String>,
                }
            })
            .is_err()
        );
    }
}
