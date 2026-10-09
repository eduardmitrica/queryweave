use quote::quote;

/// Generate the `AppendToQuery` code.
#[must_use]
pub fn code_gen(
    parsed: &crate::parse::append_to_query_shared::Parsed<'_>,
    mapping_paths: crate::mapping::append_to_query::MappingPaths,
) -> proc_macro2::TokenStream {
    let runtime = &mapping_paths.runtime;
    let crate::parse::append_to_query_shared::Parsed {
        wrapper_ident: name,
        inner_ty,
        ..
    } = parsed;
    let with_separator_wrapper_ident = mapping_paths.with_wrapper;
    let columns = mapping_paths.columns;

    quote! {
        // Every field this query may filter or sort by must have a column on
        // the entity. `Columns::push_order_by` checks it too, once instantiated; this
        // item reports it in `cargo check` as well.
        #runtime::assert_columns!(#columns, <#inner_ty as #runtime::PersistedFields>::FIELD_NAMES);

        impl #runtime::AppendToQuery for #name {
            fn append_to_query<DB, F>(
                &self,
                query_builder: &mut #runtime::QueryBuilder<'_, DB>,
                privileged_guard_option: Option<F>,
            ) where
                DB: #runtime::BindValues,
                F: FnOnce(&mut #runtime::QueryBuilder<'_, DB>, &Self) {
                    fn start_building_where_clause_if_has_options<DB: #runtime::BindValues>(
                        query_builder: &mut #runtime::QueryBuilder<'_, DB>,
                        query: &#inner_ty,
                    ) {
                        if query.and.has_options() {
                            query_builder.push("WHERE ");

                            let wrapper = #with_separator_wrapper_ident(query.and.clone());
                            #runtime::AppendToQueryWithSeparator::append_to_query(&wrapper, query_builder, "AND");
                        }
                    }
                    fn build_clauses_after_where_clause<DB: #runtime::BindValues>(
                        query_builder: &mut #runtime::QueryBuilder<'_, DB>,
                        query: &#inner_ty,
                    ) {
                        <#columns as #runtime::Columns>::push_order_by(query_builder, query.sort_keys());
                        #runtime::push_page(query_builder, query.page, query.page_size);
                    }
                    start_building_where_clause_if_has_options(query_builder, &self.0);

                    if let Some(function) = privileged_guard_option {
                        function(query_builder, &self);
                    }

                    build_clauses_after_where_clause(query_builder, &self.0);
                }
        }

        // Again as an inherent method, so callers use it without importing
        // `queryweave_sql::AppendToQuery`.
        impl #name {
            /// Append the query's `WHERE` (its filters), `ORDER BY` and
            /// `LIMIT`/`OFFSET` clauses to the query, running
            /// `privileged_guard_option` between the filters and the sorting.
            ///
            /// See the `AppendToQuery` trait's `append_to_query`.
            pub fn append_to_query<DB, F>(
                &self,
                query_builder: &mut #runtime::QueryBuilder<'_, DB>,
                privileged_guard_option: Option<F>,
            ) where
                DB: #runtime::BindValues,
                F: FnOnce(&mut #runtime::QueryBuilder<'_, DB>, &Self),
            {
                <Self as #runtime::AppendToQuery>::append_to_query(self, query_builder, privileged_guard_option);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_code_gen() {
        let parsed = crate::parse::append_to_query_shared::Parsed {
            wrapper_ident: &quote::format_ident!("OrdersQueryModelWrapper"),
            inner_ty: &syn::parse_str("OrdersQueryModel").unwrap(),
        };
        let mapping_paths = crate::mapping::append_to_query::MappingPaths {
            with_wrapper: syn::parse_str("OrdersFilterOptionsModelWrapper").unwrap(),
            columns: syn::parse_str("crate::entities::order::Order").unwrap(),
            runtime: syn::parse_quote!(::queryweave_sql),
        };

        let token_stream = super::code_gen(&parsed, mapping_paths);

        insta::assert_snapshot!(prettyplease::unparse(
            &syn::parse2::<syn::File>(token_stream).unwrap()
        ));
    }
}
