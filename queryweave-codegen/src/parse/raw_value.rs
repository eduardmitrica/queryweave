/// Struct for encapsulating the proc macro derive input for a value object.
#[derive(derive_new::new)]
pub struct Input {
    ident: syn::Ident,
    generics: syn::Generics,
    data: syn::Data,
}

/// A value object: a newtype over the single field it wraps.
#[derive(Debug, PartialEq, Eq)]
pub struct Parsed<'a> {
    pub ident: &'a syn::Ident,
    pub generics: &'a syn::Generics,
    pub inner_ty: &'a syn::Type,
}

/// Parse the input of a `RawValue` derive.
///
/// # Errors
///
/// Returns an error when the input is not a newtype struct.
pub fn parse(input: &Input) -> syn::Result<Parsed<'_>> {
    let syn::Data::Struct(data_struct) = &input.data else {
        let message = "Cannot call #[derive(RawValue)] on any other item, other than a struct";
        let help =
            "Call #[derive(RawValue)] on a newtype struct, e.g. `struct Username<T = String>(T);`";
        return crate::errors::abort_call_site(message, help);
    };

    let syn::Fields::Unnamed(unnamed) = &data_struct.fields else {
        let message = "Cannot call #[derive(RawValue)] on a struct that doesn't have exactly one unnamed field";
        let help =
            "Call #[derive(RawValue)] on a newtype struct, e.g. `struct Username<T = String>(T);`";
        return crate::errors::abort_spanned(&input.ident, message, help);
    };

    let mut fields = unnamed.unnamed.iter();
    let (Some(field), None) = (fields.next(), fields.next()) else {
        let message = "Cannot call #[derive(RawValue)] on a struct that doesn't have exactly one unnamed field";
        let help =
            "Call #[derive(RawValue)] on a newtype struct, e.g. `struct Username<T = String>(T);`";
        return crate::errors::abort_spanned(unnamed, message, help);
    };

    Ok(Parsed {
        ident: &input.ident,
        generics: &input.generics,
        inner_ty: &field.ty,
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    fn input(macro_input: proc_macro2::TokenStream) -> super::Input {
        let syn::DeriveInput {
            ident,
            generics,
            data,
            ..
        } = syn::parse2(macro_input).unwrap();

        super::Input::new(ident, generics, data)
    }

    #[test]
    fn test_parsing_a_generic_newtype_keeps_its_generics_and_inner_type() {
        let input = input(quote::quote! {
            struct Username<T = String>(T);
        });
        let parsed = super::parse(&input).unwrap();

        assert_eq!(parsed.ident, "Username");
        assert_eq!(parsed.generics.params.len(), 1);
        assert_eq!(*parsed.inner_ty, syn::parse_quote!(T));
    }

    #[test]
    fn test_parsing_a_concrete_newtype() {
        let input = input(quote::quote! {
            struct OrderId(i32);
        });
        let parsed = super::parse(&input).unwrap();

        assert!(parsed.generics.params.is_empty());
        assert_eq!(*parsed.inner_ty, syn::parse_quote!(i32));
    }

    #[rstest]
    #[case(quote::quote! { struct Empty; })]
    #[case(quote::quote! { struct Empty(); })]
    #[case(quote::quote! { struct Pair(i32, i32); })]
    #[case(quote::quote! { struct Named { value: i32 } })]
    #[case(quote::quote! { enum Kind { A } })]
    fn test_parsing_fails_on_anything_but_a_newtype(#[case] macro_input: proc_macro2::TokenStream) {
        assert!(super::parse(&input(macro_input)).is_err());
    }
}
