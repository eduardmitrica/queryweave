use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// The struct's own `NullableFields` impl, for a struct mirroring it in turn:
/// each field is nullable as its equivalent field is on the mirrored struct
/// `equivalent`, and the root of the chain is `equivalent`'s.
fn quote_nullable_fields(
    ident: &syn::Ident,
    equivalent: &syn::Path,
    field_idents: &[syn::Ident],
    equivalent_field_idents: &[syn::Ident],
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    let field_names = field_idents
        .iter()
        .map(|field| syn::LitStr::new(&field.to_string(), field.span()));
    let equivalent_names = equivalent_field_idents
        .iter()
        .map(|field| syn::LitStr::new(&field.to_string(), field.span()));
    // Spanned to the mirrored struct's path, so one that is neither a merge
    // patch model nor a mirror of one (or a cycle of mirrors) is reported
    // there rather than at the derive.
    let equivalent_runtime = crate::utils::respanned(runtime, equivalent.span());
    let nullable_fields =
        quote_spanned!(equivalent.span()=> <#equivalent as #equivalent_runtime::NullableFields>);
    let root = quote_spanned!(equivalent.span()=> <#equivalent as #equivalent_runtime::NullableFields>::Root);

    quote! {
        impl #runtime::NullableFields for #ident {
            type Root = #root;

            fn is_nullable(field: &str) -> bool {
                match field {
                    #(#field_names => #nullable_fields::is_nullable(#equivalent_names),)*
                    _ => false,
                }
            }
        }
    }
}

/// Generate the `ValidateNonNullable` code.
#[must_use]
pub fn code_gen(
    ir: crate::lower::validate_non_nullable::IntermediaryRepresentation<'_>,
    mapping_paths: crate::mapping::validate_non_nullable::MappingPaths,
) -> proc_macro2::TokenStream {
    let runtime = &mapping_paths.runtime;
    let crate::lower::validate_non_nullable::IntermediaryRepresentation {
        ident,
        field_idents,
        equivalent_field_idents,
    } = ir;
    let equivalent = mapping_paths.equivalent;

    // Every DTO field's equivalent must exist on the domain merge patch
    // model; otherwise its nullability cannot be looked up and a `null`
    // would be rejected as non-nullable at runtime. Reported on the DTO
    // field, or on the name in its `#[equivalent_field(...)]`. This check
    // is what makes the name lookup below safe.
    let fields_exist = crate::checks::assert_fields_exist(&equivalent, &equivalent_field_idents);
    let nullable_fields = quote_nullable_fields(
        ident,
        &equivalent,
        &field_idents,
        &equivalent_field_idents,
        runtime,
    );

    // The integration's `validate_non_nullable` (e.g. validator's), in its
    // library's error type, on top of `rejected_nulls`.
    let validate_non_nullable = mapping_paths
        .integration
        .as_ref()
        .map(|active| {
            active
                .integration
                .validate_non_nullable(&crate::integrations::NonNullableContext {
                    ident,
                    fields: &field_idents,
                    runtime,
                    flag: &active.flag,
                })
        })
        .unwrap_or_default();

    quote! {
        impl #ident {
            /// The fields set to `null` that may not be (by the domain merge
            /// patch model), by the names clients send: what a validation
            /// library turns into its errors.
            #[must_use]
            pub fn rejected_nulls(&self) -> Vec<&'static str> {
                let mut rejected = Vec::new();

                #(
                    {
                        // The DTO's name reports the error; the domain name
                        // (checked to exist at compile time) looks up whether
                        // the field may be set to null.
                        const FIELD_NAME: &str = stringify!(#field_idents);
                        const EQUIVALENT_FIELD_NAME: &str = stringify!(#equivalent_field_idents);
                        if !<#equivalent as #runtime::NullableFields>::is_nullable(EQUIVALENT_FIELD_NAME)
                            && #runtime::PatchKind::is_null(&self.#field_idents)
                        {
                            rejected.push(FIELD_NAME);
                        }
                    }
                )*

                rejected
            }
        }

        #validate_non_nullable

        #nullable_fields

        #fields_exist
    }
}

#[cfg(test)]
mod tests {

    /// Pretty-print the generated items, which also asserts they form a valid
    /// Rust file.
    fn pretty(token_stream: proc_macro2::TokenStream) -> String {
        prettyplease::unparse(&syn::parse2::<syn::File>(token_stream).unwrap())
    }

    fn intermediary_representation(
        ident: &syn::Ident,
    ) -> crate::lower::validate_non_nullable::IntermediaryRepresentation<'_> {
        crate::lower::validate_non_nullable::IntermediaryRepresentation {
            ident,
            // `order_number` is renamed: `#[equivalent_field(number)]`.
            field_idents: vec![
                quote::format_ident!("value"),
                quote::format_ident!("order_number"),
            ],
            equivalent_field_idents: vec![
                quote::format_ident!("value"),
                quote::format_ident!("number"),
            ],
        }
    }

    /// The mapping paths with the built-in integration named `flag`, if any.
    fn mapping_paths(flag: Option<&str>) -> crate::mapping::validate_non_nullable::MappingPaths {
        crate::mapping::validate_non_nullable::MappingPaths {
            equivalent: syn::parse_quote!(domain::models::merge_patch_order::MergePatchOrderModel),
            runtime: syn::parse_quote!(::queryweave),
            integration: flag.map(|flag| crate::integrations::Active {
                integration: crate::integrations::BUILT_IN
                    .into_iter()
                    .find(|integration| integration.name() == flag)
                    .unwrap(),
                flag: quote::format_ident!("{flag}"),
            }),
        }
    }

    // With validator: `rejected_nulls`, and `validate_non_nullable` turning it
    // into validator's errors.
    #[test]
    fn test_code_gen() {
        let ident = quote::format_ident!("MergePatchOrderDto");
        insta::assert_snapshot!(pretty(super::code_gen(
            intermediary_representation(&ident),
            mapping_paths(Some("validator"))
        )));
    }

    // Without an integration, the code names no validation library.
    #[test]
    fn test_code_gen_without_an_integration_names_no_library() {
        let ident = quote::format_ident!("MergePatchOrderDto");
        let code =
            super::code_gen(intermediary_representation(&ident), mapping_paths(None)).to_string();

        assert!(code.contains("fn rejected_nulls"));
        assert!(!code.contains("validator"), "{code}");
        assert!(!code.contains("garde"), "{code}");
    }
}
