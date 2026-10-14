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
            fn append_to_query<Q, F>(&self, query: &mut Q, privileged_guard_option: Option<F>)
            where
                Q: #runtime::ConditionsTarget + #runtime::OrderByTarget + #runtime::PageTarget,
                F: FnOnce(&mut Q, &Self) {
                    fn start_building_where_clause_if_has_options<Q: #runtime::ConditionsTarget>(
                        query: &mut Q,
                        inner: &#inner_ty,
                    ) {
                        if inner.and.has_options() {
                            #runtime::ConditionsTarget::write_where(query);

                            let wrapper = #with_separator_wrapper_ident(inner.and.clone());
                            #runtime::AppendToQueryWithSeparator::append_to_query(&wrapper, query, #runtime::Junction::And);
                        }
                    }
                    fn build_clauses_after_where_clause<Q: #runtime::OrderByTarget + #runtime::PageTarget>(
                        query: &mut Q,
                        inner: &#inner_ty,
                    ) {
                        <#columns as #runtime::Columns>::push_order_by(query, inner.sort_keys());
                        #runtime::push_page(query, inner.page, inner.page_size);
                    }
                    start_building_where_clause_if_has_options(query, &self.0);

                    if let Some(function) = privileged_guard_option {
                        function(query, &self);
                    }

                    build_clauses_after_where_clause(query, &self.0);
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
            pub fn append_to_query<Q, F>(&self, query: &mut Q, privileged_guard_option: Option<F>)
            where
                Q: #runtime::ConditionsTarget + #runtime::OrderByTarget + #runtime::PageTarget,
                F: FnOnce(&mut Q, &Self),
            {
                <Self as #runtime::AppendToQuery>::append_to_query(self, query, privileged_guard_option);
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
