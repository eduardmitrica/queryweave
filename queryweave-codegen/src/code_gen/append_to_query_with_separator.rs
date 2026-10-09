use quote::quote;

/// Generate the `AppendToQueryWithSeparator` code.
#[must_use]
pub fn code_gen(
    parsed: &crate::parse::append_to_query_shared::Parsed<'_>,
    mapping_paths: crate::mapping::append_to_query_with_separator::MappingPaths,
) -> proc_macro2::TokenStream {
    let runtime = &mapping_paths.runtime;
    let crate::parse::append_to_query_shared::Parsed {
        wrapper_ident: name,
        inner_ty,
    } = parsed;
    let columns = mapping_paths.columns;

    quote! {
        impl #runtime::AppendToQueryWithSeparator for #name {
            fn append_to_query<DB: #runtime::BindValues>(
                &self,
                query_builder: &mut #runtime::QueryBuilder<'_, DB>,
                separator: &str,
            ) {
                // The domain model describes its set filters as data; turning
                // them into SQL, against the entity's columns, happens here.
                <#columns as #runtime::Columns>::push_conditions(query_builder, self.0.conditions(), separator);
            }
        }

        // Again as an inherent method, so callers use it without importing
        // `queryweave_sql::AppendToQueryWithSeparator`.
        impl #name {
            /// Append the set filters to the query as conditions joined by
            /// `separator` (e.g. `AND`).
            ///
            /// See the `AppendToQueryWithSeparator` trait's `append_to_query`.
            pub fn append_to_query<DB: #runtime::BindValues>(
                &self,
                query_builder: &mut #runtime::QueryBuilder<'_, DB>,
                separator: &str,
            ) {
                <Self as #runtime::AppendToQueryWithSeparator>::append_to_query(self, query_builder, separator);
            }
        }

        // Every field these filters may name must have a column on the
        // entity. `Columns::push_conditions` checks it too, once instantiated; this
        // item reports it in `cargo check` as well.
        #runtime::assert_columns!(#columns, <#inner_ty as #runtime::PersistedFields>::FIELD_NAMES);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_code_gen() {
        let parsed = crate::parse::append_to_query_shared::Parsed {
            wrapper_ident: &quote::format_ident!("OrdersFilterOptionsModelWrapper"),
            inner_ty: &syn::parse_str("OrdersFilterOptionsModel").unwrap(),
        };
        let mapping_paths = crate::mapping::append_to_query_with_separator::MappingPaths {
            columns: syn::parse_str("crate::entities::order::Order").unwrap(),
            runtime: syn::parse_quote!(::queryweave_sql),
        };

        let token_stream = super::code_gen(&parsed, mapping_paths);

        insta::assert_snapshot!(prettyplease::unparse(
            &syn::parse2::<syn::File>(token_stream).unwrap()
        ));
    }
}
