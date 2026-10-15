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
    let conditions_wrapper = mapping_paths.with_wrapper;
    let columns = mapping_paths.columns;
    let diesel = diesel(name, inner_ty, &conditions_wrapper, &columns, runtime);

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

                            let wrapper = #conditions_wrapper(inner.and.clone());
                            #runtime::AppendConditions::append_conditions(&wrapper, query, #runtime::Junction::And);
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

        #diesel
    }
}

/// The wrapper's Diesel method: the query's filters, sorting and page
/// applied to a Diesel boxed query of a Diesel entity (its
/// `diesel::Entity`), let through the runtime's `if_diesel!` like the
/// entity's own Diesel methods.
fn diesel(
    name: &syn::Ident,
    inner_ty: &syn::Type,
    conditions_wrapper: &syn::Path,
    columns: &syn::Path,
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    let diesel = quote! { #runtime::diesel };
    let entity = quote! { <#columns as #diesel::Entity<'q, DB>> };

    quote! {
        #runtime::__private::if_diesel! {
            impl #name {
                /// The boxed query `query` (`table.into_boxed()`, after any
                /// `select` or `filter` of its own) with this query's filters
                /// (`AND`ed to the query's own conditions), its sorting (after
                /// the query's own) and its page (`LIMIT` / `OFFSET`).
                ///
                /// Builds only for a Diesel entity (one deriving `Columns`
                /// with `#[diesel(table_name = ...)]`). Add a privileged
                /// guard as a `filter` of `query`.
                ///
                /// # Errors
                ///
                /// The `ValueMismatch` of a filter value that does not fit
                /// its column's SQL type.
                pub fn diesel_apply_to<'q, DB, Q>(
                    &self,
                    query: Q,
                ) -> ::core::result::Result<Q, #diesel::ValueMismatch>
                where
                    #columns: #diesel::Entity<'q, DB>,
                    Q: #diesel::BoxedQuery<'q, #entity::Table, DB>,
                {
                    let inner: &#inner_ty = &self.0;
                    let condition = #conditions_wrapper(inner.and.clone())
                        .diesel_condition::<DB>(#runtime::Junction::And)?;

                    ::core::result::Result::Ok(#diesel::apply(
                        query,
                        condition,
                        #entity::order_by(inner.sort_keys()),
                        #diesel::page(inner.page, inner.page_size),
                    ))
                }
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
