use quote::quote;

/// Generate the `Columns` code.
#[must_use]
pub fn code_gen(
    parsed: &crate::parse::columns::Parsed<'_>,
    mapping_paths: crate::mapping::columns::MappingPaths,
) -> proc_macro2::TokenStream {
    let runtime = &mapping_paths.runtime;
    let crate::parse::columns::Parsed {
        ident,
        generics,
        columns,
    } = parsed;
    let domain_model = mapping_paths.domain_model;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let name = ident.to_string();
    let domain_fields = columns.iter().map(|column| column.domain_field.to_string());
    let column_names = columns.iter().map(|column| &column.name);

    // A domain field named by `#[domain_field(name)]` must exist on the
    // domain model; reported at the name. (Same-named ones need no check:
    // a domain field without a column is caught where the model's fields
    // are used, by the `queryweave_sql` SQL helpers.)
    let fields_exist = crate::checks::assert_fields_exist(
        &domain_model,
        columns
            .iter()
            .filter(|column| column.is_mapped)
            .map(|column| &column.domain_field),
    );

    // The `SET` writer, with merge patches only (it writes their data).
    let assignments = if cfg!(feature = "merge-patch") {
        quote! {
            /// Push `column = value` assignments of the model `M` (a `MergePatch`
            /// model's `changed_values`) for an `UPDATE … SET`, comma separated,
            /// returning the separated builder so more can be added.
            ///
            /// See the `Columns` trait's `push_assignments`.
            pub fn push_assignments<'qb, 'args, DB, M>(
                query_builder: &'qb mut #runtime::QueryBuilder<'args, DB>,
                assignments: #runtime::FieldValues<M>,
            ) -> #runtime::Separated<'qb, 'args, DB, &'static str>
            where
                DB: #runtime::BindValues,
                M: #runtime::PersistedFields,
            {
                const { #runtime::__private::assert_model_columns::<Self, M>() };
                #runtime::__private::push_assignments::<Self, DB, M>(query_builder, assignments)
            }
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    quote! {
        impl #impl_generics #runtime::Columns for #ident #type_generics #where_clause {
            const NAME: &'static str = #name;
            const COLUMNS: &'static [(&'static str, &'static str)] = &[
                #((#domain_fields, #column_names)),*
            ];
        }

        // The SQL methods again as inherent ones, so callers use them without
        // importing `queryweave_sql::Columns`. Each runs the column check
        // itself, so a missing column is reported at its caller (the line
        // writing the data), then calls the unchecked writer.
        impl #impl_generics #ident #type_generics #where_clause {
            /// Push the conditions of the model `M` (a generated filter-options
            /// model's `conditions()`) to a query, joined by `separator`, each
            /// against the column this entity stores its domain field in.
            ///
            /// See the `Columns` trait's `push_conditions`.
            pub fn push_conditions<DB, M>(
                query_builder: &mut #runtime::QueryBuilder<'_, DB>,
                conditions: #runtime::Conditions<M>,
                separator: &str,
            ) where
                DB: #runtime::BindValues,
                M: #runtime::PersistedFields,
            {
                const { #runtime::__private::assert_model_columns::<Self, M>() };
                #runtime::__private::push_conditions::<Self, DB, M>(query_builder, conditions, separator);
            }

            /// Push an `ORDER BY` clause for the sort keys of the model `M` (a
            /// generated query model's `sort_keys()`), each by the column this
            /// entity stores its domain field in; nothing when there are none.
            ///
            /// See the `Columns` trait's `push_order_by`.
            pub fn push_order_by<DB, M>(
                query_builder: &mut #runtime::QueryBuilder<'_, DB>,
                sort_keys: #runtime::SortKeys<M>,
            ) where
                DB: #runtime::BindValues,
                M: #runtime::PersistedFields,
            {
                const { #runtime::__private::assert_model_columns::<Self, M>() };
                #runtime::__private::push_order_by::<Self, DB, M>(query_builder, sort_keys);
            }

            #assignments
        }

        #fields_exist
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_code_gen() {
        let syn::DeriveInput {
            ident,
            generics,
            data,
            attrs,
            ..
        } = syn::parse2(quote::quote! {
            #[derive(sqlx::FromRow, Columns)]
            #[columns(model = domain::models::order::OrderModel)]
            struct Order {
                id: i32,
                #[domain_field(delivery_date)]
                delivered_at: DateTime<Utc>,
                #[sqlx(rename = "order_number")]
                number: String,
            }
        })
        .unwrap();
        let input = crate::parse::columns::Input::new(ident, generics, data, attrs);
        let mapping_paths = crate::mapping::columns::MappingPaths {
            domain_model: syn::parse_str("domain::models::order::OrderModel").unwrap(),
            runtime: syn::parse_quote!(::queryweave_sql),
        };

        let token_stream = super::code_gen(
            &crate::parse::columns::parse(&input).unwrap(),
            mapping_paths,
        );

        insta::assert_snapshot!(prettyplease::unparse(
            &syn::parse2::<syn::File>(token_stream).unwrap()
        ));
    }
}
