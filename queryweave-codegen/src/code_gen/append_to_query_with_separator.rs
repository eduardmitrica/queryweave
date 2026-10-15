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
    let diesel = diesel(name, &columns, runtime);

    quote! {
        impl #runtime::AppendToQueryWithSeparator for #name {
            fn append_to_query<Q: #runtime::ConditionsTarget>(&self, query: &mut Q, junction: #runtime::Junction) {
                // The domain model describes its set filters as data; turning
                // them into SQL, against the entity's columns, happens here.
                <#columns as #runtime::Columns>::push_conditions(query, self.0.conditions(), junction);
            }
        }

        // Again as an inherent method, so callers use it without importing
        // `queryweave_sql::AppendToQueryWithSeparator`.
        impl #name {
            /// Append the set filters to the query as conditions joined by
            /// `junction`.
            ///
            /// See the `AppendToQueryWithSeparator` trait's `append_to_query`.
            pub fn append_to_query<Q: #runtime::ConditionsTarget>(&self, query: &mut Q, junction: #runtime::Junction) {
                <Self as #runtime::AppendToQueryWithSeparator>::append_to_query(self, query, junction);
            }
        }

        // Every field these filters may name must have a column on the
        // entity. `Columns::push_conditions` checks it too, once instantiated; this
        // item reports it in `cargo check` as well.
        #runtime::assert_columns!(#columns, <#inner_ty as #runtime::PersistedFields>::FIELD_NAMES);

        #diesel
    }
}

/// The wrapper's Diesel method: the set filters as one Diesel condition, for
/// an entity with `#[diesel(table_name)]` (its `diesel::Entity`), let through
/// the runtime's `if_diesel!` like the entity's own Diesel methods.
fn diesel(name: &syn::Ident, columns: &syn::Path, runtime: &syn::Path) -> proc_macro2::TokenStream {
    let diesel = quote! { #runtime::diesel };

    quote! {
        #runtime::__private::if_diesel! {
            impl #name {
                /// The set filters as one Diesel condition on the entity's
                /// table, joined by `junction`, for a boxed query's `filter`
                /// (or `or_filter`); `None` when none are set.
                ///
                /// Builds only for a Diesel entity (one deriving `Columns`
                /// with `#[diesel(table_name = ...)]`).
                ///
                /// # Errors
                ///
                /// The `ValueMismatch` of a value that does not fit its
                /// column's SQL type.
                pub fn diesel_condition<'q, DB>(
                    &self,
                    junction: #runtime::Junction,
                ) -> ::core::result::Result<
                    ::core::option::Option<
                        #diesel::BoxedCondition<'q, <#columns as #diesel::Entity<'q, DB>>::Table, DB>,
                    >,
                    #diesel::ValueMismatch,
                >
                where
                    #columns: #diesel::Entity<'q, DB>,
                {
                    <#columns as #diesel::Entity<'q, DB>>::conditions(self.0.conditions(), junction)
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
