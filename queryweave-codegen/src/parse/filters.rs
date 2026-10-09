use syn::Fields;

/// Struct for encapsulating the proc macro derive input.
#[derive(derive_new::new)]
pub struct Input {
    ident: syn::Ident,
    data: syn::Data,
    derive_name: &'static str,
}

/// Parsing output.
pub struct Parsed<'a> {
    /// The names of the generated types, from the derive's conventions.
    pub idents: crate::conventions::FiltersNames,
    pub named_fields: &'a syn::FieldsNamed,
}

/// Parse the input of a `Filters` derive (domain or API).
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

    let idents = C::filters_names(&input.ident)?;

    let Fields::Named(named_fields) = &data_struct.fields else {
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
        idents,
        named_fields,
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parsing_succeeds_when_all_preconditions_are_met() {
        let macro_input = quote::quote! {
            #[derive(Filters)]
            struct UserModel {
                #[filter(sw)]
                #[sortable]
                username: String,
            }
        };

        let syn::DeriveInput { ident, data, .. } = syn::parse2(macro_input).unwrap();

        let input = super::Input::new(ident, data, "Filters");
        let parsing_result = super::parse::<crate::conventions::DefaultConventions>(&input);

        assert!(parsing_result.is_ok());

        let super::Parsed {
            idents,
            named_fields,
        } = parsing_result.unwrap();

        let expected_idents = crate::conventions::FiltersNames {
            query: quote::format_ident!("UsersQueryModel"),
            filters: quote::format_ident!("UsersFilterOptionsModel"),
            sort_field: quote::format_ident!("UserSortFieldModel"),
            sort_details: quote::format_ident!("UserSortDetailsModel"),
            input: quote::format_ident!("UserModel"),
            page: quote::format_ident!("UsersPageModel"),
            query_params: quote::format_ident!("UsersQueryParams"),
        };

        insta::assert_debug_snapshot!(idents);
        assert_eq!(expected_idents, idents);

        let expected_named_fields_macro_input = quote::quote! {
            {
                #[filter(sw)]
                #[sortable]
                username: String,
            }
        };
        let expected_named_fields: syn::FieldsNamed =
            syn::parse2(expected_named_fields_macro_input).unwrap();

        insta::assert_debug_snapshot!(*named_fields);
        assert_eq!(expected_named_fields, *named_fields);
    }

    #[test]
    fn test_parsing_fails_when_given_incorrect_struct_name() {
        let macro_input = quote::quote! {
            #[derive(Filters)]
            struct User {
                #[filter(sw)]
                #[sortable]
                username: String,
            }
        };

        let syn::DeriveInput { ident, data, .. } = syn::parse2(macro_input).unwrap();

        let input = super::Input::new(ident, data, "Filters");
        let parsed_result = super::parse::<crate::conventions::DefaultConventions>(&input);

        assert!(parsed_result.is_err());
    }

    /// Conventions naming the generated types after the whole struct name,
    /// which may then have a single segment.
    struct SuffixConventions;

    impl crate::conventions::Conventions for SuffixConventions {
        fn filters_names(input: &syn::Ident) -> syn::Result<crate::conventions::FiltersNames> {
            Ok(crate::conventions::FiltersNames {
                query: quote::format_ident!("{input}Search"),
                filters: quote::format_ident!("{input}Filters"),
                sort_field: quote::format_ident!("{input}SortKey"),
                sort_details: quote::format_ident!("{input}Sort"),
                input: input.clone(),
                page: quote::format_ident!("{input}Page"),
                query_params: quote::format_ident!("{input}SearchParams"),
            })
        }

        fn check_model_name(_name: &syn::Ident) -> syn::Result<()> {
            Ok(())
        }

        fn check_merge_patch_name<S: quote::ToTokens + ?Sized>(
            _name: &syn::Ident,
            _span: &S,
            _what: &str,
        ) -> syn::Result<()> {
            Ok(())
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

    // The names come from the derive's conventions, so other conventions name
    // the generated types differently and accept other struct names.
    #[test]
    fn test_parsing_names_the_generated_types_after_the_conventions() {
        let macro_input = quote::quote! {
            struct User {
                #[filter(sw)]
                username: String,
            }
        };

        let syn::DeriveInput { ident, data, .. } = syn::parse2(macro_input).unwrap();
        let input = super::Input::new(ident, data, "Filters");

        let parsed = super::parse::<SuffixConventions>(&input).unwrap();

        assert_eq!(parsed.idents.query, quote::format_ident!("UserSearch"));
        assert_eq!(
            parsed.idents.sort_field,
            quote::format_ident!("UserSortKey")
        );
        // The default conventions need a second segment.
        assert!(super::parse::<crate::conventions::DefaultConventions>(&input).is_err());
    }
}
