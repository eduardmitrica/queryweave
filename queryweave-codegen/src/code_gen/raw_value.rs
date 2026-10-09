use quote::quote;

/// Generate the `RawValue` code.
#[must_use]
pub fn code_gen(
    parsed: &crate::parse::raw_value::Parsed<'_>,
    mapping_paths: &crate::mapping::raw_value::MappingPaths,
) -> proc_macro2::TokenStream {
    let runtime = &mapping_paths.runtime;
    let crate::parse::raw_value::Parsed {
        ident,
        generics,
        inner_ty,
    } = *parsed;

    // The value object's raw value is its inner type's raw value, so a
    // generic value object is `RawValue` whenever what it wraps is.
    let mut generics = generics.clone();
    generics
        .make_where_clause()
        .predicates
        .push(syn::parse_quote!(#inner_ty: #runtime::RawValue));
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #runtime::RawValue for #ident #type_generics #where_clause {
            type Raw = <#inner_ty as #runtime::RawValue>::Raw;

            fn to_raw(&self) -> Self::Raw {
                #runtime::RawValue::to_raw(&self.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    fn code_gen(macro_input: proc_macro2::TokenStream) -> String {
        let syn::DeriveInput {
            ident,
            generics,
            data,
            ..
        } = syn::parse2(macro_input).unwrap();
        let input = crate::parse::raw_value::Input::new(ident, generics, data);
        let token_stream = super::code_gen(
            &crate::parse::raw_value::parse(&input).unwrap(),
            &crate::mapping::raw_value::MappingPaths {
                runtime: syn::parse_quote!(::queryweave),
            },
        );

        prettyplease::unparse(&syn::parse2::<syn::File>(token_stream).unwrap())
    }

    #[test]
    fn test_code_gen_for_a_generic_newtype() {
        insta::assert_snapshot!(code_gen(quote::quote! {
            struct Username<T = String>(T);
        }));
    }

    #[test]
    fn test_code_gen_for_a_concrete_newtype() {
        insta::assert_snapshot!(code_gen(quote::quote! {
            struct OrderId(i32);
        }));
    }
}
