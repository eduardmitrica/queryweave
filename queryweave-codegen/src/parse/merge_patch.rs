#[derive(derive_new::new)]
pub struct Input {
    pub ident: syn::Ident,
    data: syn::Data,
    derive_name: &'static str,
}

/// Parsing output.
pub struct Parsed<'a> {
    /// Identifiers used in the code generation step.
    pub ident: &'a syn::Ident,
    pub named_fields: &'a syn::FieldsNamed,
}

/// Parse the input of a `MergePatch` derive.
///
/// # Errors
///
/// Returns an error when the input is not a struct with named fields, or its
/// name does not fit the conventions `C`.
pub fn parse<C: crate::conventions::Conventions>(input: &Input) -> syn::Result<Parsed<'_>> {
    let syn::Data::Struct(data_struct) = &input.data else {
        let message = format!(
            "Cannot call #[derive({})] on any other item, other than a struct",
            input.derive_name
        );
        let help = format!("Call #[derive({})] on a struct", input.derive_name);
        return crate::errors::abort_call_site(message.as_str(), help.as_str());
    };

    C::check_model_name(&input.ident)?;

    let syn::Fields::Named(named_fields) = &data_struct.fields else {
        let message = format!(
            "Cannot call #[derive({})] on a struct that doesn't have named fields",
            input.derive_name
        );
        let help = format!(
            "Call #[derive({})] on a struct with named fields",
            input.derive_name
        );
        return crate::errors::abort_spanned(&data_struct.fields, message.as_str(), help.as_str());
    };

    Ok(Parsed {
        ident: &input.ident,
        named_fields,
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parsing_succeeds_when_all_preconditions_are_met() {
        let macro_input = quote::quote! {
            #[derive(MergePatch)]
            #[merge_patch(patch = MergePatchOrderModel)]
            struct OrderModel {
                client: Option<String>,

                #[merge_patch(skip)]
                created_by_user_with_id: u32,
            }
        };

        let syn::DeriveInput { ident, data, .. } = syn::parse2(macro_input).unwrap();

        let input = super::Input::new(ident, data, "MergePatch");
        let parsing_result = super::parse::<crate::conventions::DefaultConventions>(&input);

        assert!(parsing_result.is_ok());

        let super::Parsed {
            ident,
            named_fields,
        } = parsing_result.unwrap();

        let expected_ident = &quote::format_ident!("OrderModel");

        insta::assert_debug_snapshot!(ident);
        assert_eq!(expected_ident, ident);

        let expected_named_fields_macro_input = quote::quote! {
            {
                client: Option<String>,

                #[merge_patch(skip)]
                created_by_user_with_id: u32,
            }
        };
        let expected_named_fields: syn::FieldsNamed =
            syn::parse2(expected_named_fields_macro_input).unwrap();

        insta::assert_debug_snapshot!(named_fields);
        assert_eq!(expected_named_fields, *named_fields);
    }

    #[test]
    fn test_parsing_fails_when_given_incorrect_struct_name() {
        let macro_input = quote::quote! {
            #[derive(MergePatch)]
            struct User {
                client: Option<String>,

                #[merge_patch(skip)]
                created_by_user_with_id: u32,
            }
        };

        let syn::DeriveInput { ident, data, .. } = syn::parse2(macro_input).unwrap();

        let input = super::Input::new(ident, data, "MergePatch");
        let parsed_result = super::parse::<crate::conventions::DefaultConventions>(&input);

        assert!(parsed_result.is_err());
    }
}
