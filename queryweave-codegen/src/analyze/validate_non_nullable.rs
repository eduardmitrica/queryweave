#[derive(Default, Debug, PartialEq, Eq, derive_new::new)]
pub struct Model {
    #[new(default)]
    pub field_idents: Vec<syn::Ident>,
    /// The ident each DTO field (same index in `field_idents`) has on the
    /// domain merge patch model (see `#[equivalent_field(name)]`), carrying the
    /// span of what the user wrote.
    #[new(default)]
    pub equivalent_field_idents: Vec<syn::Ident>,
}

/// Pair each field of a merge patch DTO with its name on the domain merge
/// patch model.
///
/// # Errors
///
/// Returns the errors of malformed `#[equivalent_field(...)]` attributes, and
/// of `Option<Option<T>>` fields (a field that may be set to `null` is a
/// `queryweave::Patch<T>`).
pub fn analyze(parsed: &crate::parse::validate_non_nullable::Parsed<'_>) -> syn::Result<Model> {
    let mut model = Model::new();
    let mut errors = crate::errors::Errors::default();

    // Every DTO field is checked against the domain merge patch model's
    // nullable fields, under the name it has there; its type needs no
    // classification.
    for field in &parsed.named_fields.named {
        let Some(field_ident) = &field.ident else {
            continue;
        };

        // A field that may be set to `null` is a `queryweave::Patch<T>`; an
        // `Option<Option<T>>` would never report its `null`.
        errors.check(crate::attributes::reject_option_of_option(field));
        if let Some(equivalent) = errors.check(crate::attributes::equivalent_field_name(field)) {
            model.field_idents.push(field_ident.clone());
            model.equivalent_field_idents.push(equivalent);
        }
    }

    errors.finish(model)
}

#[cfg(test)]
mod tests {

    fn analyze_named_fields(
        named_fields_input: proc_macro2::TokenStream,
    ) -> syn::Result<super::Model> {
        let named_fields = &syn::parse2(named_fields_input).unwrap();
        let parsed = crate::parse::validate_non_nullable::Parsed {
            ident: &quote::format_ident!("MergePatchOrderDto"),
            named_fields,
        };

        super::analyze(&parsed)
    }

    // Each field keeps its own name, and has the domain name of
    // `#[equivalent_field(name)]`, or its own without it.
    #[test]
    fn test_analysis_records_each_fields_equivalent_name() {
        let model = analyze_named_fields(quote::quote! {
            {
                pub value: Patch<f64>,
                #[equivalent_field(number)]
                pub order_number: Patch<String>,
            }
        })
        .unwrap();

        let expected_model = super::Model {
            field_idents: vec![
                quote::format_ident!("value"),
                quote::format_ident!("order_number"),
            ],
            equivalent_field_idents: vec![
                quote::format_ident!("value"),
                quote::format_ident!("number"),
            ],
        };
        pretty_assertions::assert_eq!(expected_model, model);
    }

    #[test]
    fn test_analysis_fails_on_a_malformed_equivalent_field() {
        assert!(
            analyze_named_fields(quote::quote! {
                {
                    #[equivalent_field()]
                    pub order_number: Patch<String>,
                }
            })
            .is_err()
        );
    }

    // `Option<Option<T>>` is not a merge patch field: its `null` would never
    // be reported. The error names `Patch<T>`.
    #[test]
    fn test_analysis_rejects_option_of_option() {
        let error = analyze_named_fields(quote::quote! {
            {
                pub value: Option<Option<f64>>,
            }
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("Merge patches do not support `Option<Option<T>>`"));
        assert!(error.contains("queryweave::Patch<T>"));
    }
}
