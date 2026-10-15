#[derive(derive_new::new)]
pub struct Input {
    ident: syn::Ident,
    data: syn::Data,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Parsed<'a> {
    pub wrapper_ident: &'a syn::Ident,
    pub inner_ty: &'a syn::Type,
}

/// Parse the input of an `AppendToQuery` or `AppendConditions`
/// derive.
///
/// # Errors
///
/// Returns an error when the input is not a newtype struct.
pub fn parse(input: &Input) -> syn::Result<Parsed<'_>> {
    let name = &input.ident;

    let syn::Data::Struct(data_struct) = &input.data else {
        let message = "Cannot call #[derive(AppendConditions)] or #[derive(AppendToQuery)] on any other item, other than a struct";
        let help = "Call #[derive(AppendConditions)] or #[derive(AppendToQuery)] on a struct";
        return crate::errors::abort_call_site(message, help);
    };
    let syn::Fields::Unnamed(unnamed) = &data_struct.fields else {
        let message = "Cannot call #[derive(AppendConditions)] or #[derive(AppendToQuery)] on a struct that doesn't have unnamed fields";
        let help = "Call #[derive(AppendConditions)] or #[derive(AppendToQuery)] on a struct with unnamed fields";
        return crate::errors::abort_spanned(&data_struct.fields, message, help);
    };

    if unnamed.unnamed.len() != 1 {
        let message = "Cannot call #[derive(AppendConditions)] or #[derive(AppendToQuery)] on a struct that doesn't have exactly one unnamed field";
        let help = "Call #[derive(AppendConditions)] or #[derive(AppendToQuery)] on a struct with exactly one unnamed field";
        return crate::errors::abort_spanned(&unnamed.unnamed, message, help);
    }

    let Some(field) = &unnamed.unnamed.first() else {
        let message = "Cannot call #[derive(AppendConditions)] or #[derive(AppendToQuery)] on a struct that doesn't have exactly one unnamed field";
        let help = "Call #[derive(AppendConditions)] or #[derive(AppendToQuery)] on a struct with exactly one unnamed field";
        return crate::errors::abort_spanned(&unnamed.unnamed, message, help);
    };

    Ok(Parsed {
        wrapper_ident: name,
        inner_ty: &field.ty,
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_parsing_succeeds_when_all_preconditions_are_met() {
        let macro_input = quote::quote! {
            #[derive(Debug, AppendConditions)]
            pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);
        };

        let syn::DeriveInput { ident, data, .. } = syn::parse2(macro_input).unwrap();

        let input = super::Input::new(ident, data);
        let parsing_result = super::parse(&input);

        assert!(parsing_result.is_ok());

        let parsed = parsing_result.unwrap();

        let expected_wrapper_ident = &quote::format_ident!("OrdersFilterOptionsModelWrapper");
        let expected_inner_ident = &syn::parse_str("OrdersFilterOptionsModel").unwrap();
        let expected_parsed = super::Parsed {
            wrapper_ident: expected_wrapper_ident,
            inner_ty: expected_inner_ident,
        };

        insta::assert_debug_snapshot!(parsed);
        pretty_assertions::assert_eq!(expected_parsed, parsed);
    }

    #[test]
    fn test_parsing_fails_when_no_unnamed_fields_provided() {
        let macro_input = quote::quote! {
            #[derive(Debug, AppendConditions)]
            pub struct OrdersFilterOptionsModelWrapper();
        };

        let syn::DeriveInput { ident, data, .. } = syn::parse2(macro_input).unwrap();

        let input = super::Input::new(ident, data);
        let parsed_result = super::parse(&input);

        assert!(parsed_result.is_err());
    }

    #[test]
    fn test_parsing_fails_when_more_than_one_unnamed_field_provided() {
        let macro_input = quote::quote! {
            #[derive(Debug, AppendConditions)]
            pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel, pub UserFilterOptionsModel);
        };

        let syn::DeriveInput { ident, data, .. } = syn::parse2(macro_input).unwrap();

        let input = super::Input::new(ident, data);
        let parsed_result = super::parse(&input);

        assert!(parsed_result.is_err());
    }
}
