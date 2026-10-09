/// The struct attributes of a `MergePatchNullableFields` merge patch model.
#[derive(Debug, PartialEq, Eq)]
pub struct TopLevelMeta {
    /// `#[queryweave(crate = path)]`: where the runtime crate the generated
    /// code names lives, overriding the conventions' default.
    pub runtime_crate: Option<syn::Path>,
}

impl crate::steps::TopLevelMeta for TopLevelMeta {
    fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self> {
        Ok(Self {
            runtime_crate: crate::attributes::crate_option(attributes)?,
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
    fn test_parsing_top_level_meta_reads_the_runtime_crate() {
        pretty_assertions::assert_eq!(
            parse(quote::quote! {
                #[queryweave(crate = my::runtime)]
                struct Item;
            })
            .unwrap(),
            super::TopLevelMeta {
                runtime_crate: Some(syn::parse_quote!(my::runtime)),
            }
        );
        pretty_assertions::assert_eq!(
            parse(quote::quote! { struct Item; }).unwrap(),
            super::TopLevelMeta {
                runtime_crate: None
            }
        );
    }

    #[test]
    fn test_parsing_top_level_meta_fails_for_a_malformed_crate_option() {
        for item in [
            quote::quote! { #[queryweave()] struct Item; },
            quote::quote! { #[queryweave(crate)] struct Item; },
            quote::quote! { #[queryweave(krate = my::runtime)] struct Item; },
        ] {
            assert!(parse(item).is_err());
        }
    }
}
