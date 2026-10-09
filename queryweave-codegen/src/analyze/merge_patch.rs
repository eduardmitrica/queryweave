#[derive(Debug, Default, PartialEq, Eq, derive_new::new)]
pub struct Model {
    /// The patchable fields, i.e. those not marked `#[merge_patch(skip)]`.
    #[new(default)]
    pub field_idents: Vec<syn::Ident>,
}

/// Analyze the fields of a `MergePatch` model.
///
/// # Errors
///
/// Returns the errors of malformed `#[merge_patch(...)]` attributes.
pub fn analyze(parsed: &crate::parse::merge_patch::Parsed<'_>) -> syn::Result<Model> {
    let mut model = Model::new();
    let mut errors = crate::errors::Errors::default();

    for field in &parsed.named_fields.named {
        let Some(field_ident) = &field.ident else {
            continue;
        };

        // A skipped field, or a malformed attribute: either way, the field
        // is not patchable. What a patchable field takes (an `Option`, or a
        // `Patch` for an `Option` field) is the compiler's to check, through
        // `queryweave::PatchField`.
        if errors.check(crate::attributes::is_skipped(field, "merge_patch")) == Some(false) {
            model.field_idents.push(field_ident.clone());
        }
    }

    errors.finish(model)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    fn analyze_named_fields(
        named_fields_input: proc_macro2::TokenStream,
    ) -> syn::Result<super::Model> {
        let named_fields = &syn::parse2(named_fields_input).unwrap();
        let parsed = crate::parse::merge_patch::Parsed {
            ident: &quote::format_ident!("OrderModel"),
            named_fields,
        };

        super::analyze(&parsed)
    }

    // Every field but the skipped ones is patchable, whatever its type: value
    // objects need no generic argument, since nothing is classified by name.
    #[test]
    fn test_analysis_keeps_the_fields_that_are_not_skipped() {
        let model = analyze_named_fields(quote::quote! {
            {
                client: Option<String>,
                capacity: OrderCapacity,

                #[merge_patch(skip)]
                created_by_user_with_id: u32,
            }
        })
        .unwrap();

        pretty_assertions::assert_eq!(
            model.field_idents,
            vec![
                quote::format_ident!("client"),
                quote::format_ident!("capacity"),
            ]
        );
    }

    #[rstest]
    // Malformed `#[merge_patch(...)]` attributes.
    #[case(quote::quote! { { #[merge_patch()] client: Option<String>, } })]
    #[case(quote::quote! { { #[merge_patch] client: Option<String>, } })]
    #[case(quote::quote! { { #[merge_patch(ignore)] client: Option<String>, } })]
    fn test_analysis_fails(#[case] named_fields_input: proc_macro2::TokenStream) {
        assert!(analyze_named_fields(named_fields_input).is_err());
    }
}
