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
        diesel_table,
    } = parsed;
    let domain_model = mapping_paths.domain_model;
    let diesel = diesel_table
        .as_ref()
        .map(|table| {
            let items = diesel_items(parsed, table, runtime);
            quote! { #runtime::__private::if_diesel! { #items } }
        })
        .unwrap_or_default();
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

    // The `SET` writer, with merge patches only (it writes their data): the
    // runtime's gate, since it names the runtime's merge-patch items.
    let assignments = quote! {
        #runtime::__private::if_merge_patch! {
            /// Push `column = value` assignments of the model `M` (a `MergePatch`
            /// model's `changed_values`) for an `UPDATE … SET`, comma separated,
            /// returning what the query gives to add more (for sqlx, the
            /// separated builder).
            ///
            /// See the `Columns` trait's `push_assignments`.
            pub fn push_assignments<Q, M>(
                query: &mut Q,
                assignments: #runtime::FieldValues<M>,
            ) -> <Q as #runtime::AssignmentsTarget>::Assignments<'_>
            where
                Q: #runtime::AssignmentsTarget,
                M: #runtime::PersistedFields,
            {
                const { #runtime::__private::assert_model_columns::<Self, M>() };
                #runtime::AssignmentsTarget::write_assignments::<Self, M>(query, assignments)
            }
        }
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
        // writing the data), then calls the query's unchecked writer.
        impl #impl_generics #ident #type_generics #where_clause {
            /// Push the conditions of the model `M` (a generated filter-options
            /// model's `conditions()`) to a query, joined by `junction`, each
            /// against the column this entity stores its domain field in.
            ///
            /// See the `Columns` trait's `push_conditions`.
            pub fn push_conditions<Q, M>(
                query: &mut Q,
                conditions: #runtime::Conditions<M>,
                junction: #runtime::Junction,
            ) where
                Q: #runtime::ConditionsTarget,
                M: #runtime::PersistedFields,
            {
                const { #runtime::__private::assert_model_columns::<Self, M>() };
                #runtime::ConditionsTarget::write_conditions::<Self, M>(query, conditions, junction);
            }

            /// Push an `ORDER BY` clause for the sort keys of the model `M` (a
            /// generated query model's `sort_keys()`), each by the column this
            /// entity stores its domain field in; nothing when there are none.
            ///
            /// See the `Columns` trait's `push_order_by`.
            pub fn push_order_by<Q, M>(
                query: &mut Q,
                sort_keys: #runtime::SortKeys<M>,
            ) where
                Q: #runtime::OrderByTarget,
                M: #runtime::PersistedFields,
            {
                const { #runtime::__private::assert_model_columns::<Self, M>() };
                #runtime::OrderByTarget::write_order_by::<Self, M>(query, sort_keys);
            }

            #assignments
        }

        #fields_exist

        #diesel
    }
}

/// The column match of the Diesel methods: each column's domain field (a
/// string to match the data's field against) and its column in `table!`,
/// carrying the user's span, so rustc reports a column missing from `table!`
/// at the entity field naming it.
fn diesel_arms<'p>(
    parsed: &'p crate::parse::columns::Parsed<'_>,
) -> (Vec<String>, Vec<&'p syn::Ident>) {
    parsed
        .columns
        .iter()
        .map(|column| (column.domain_field.to_string(), &column.diesel_ident))
        .unzip()
}

/// `(domain field, what its column takes)` for every column, the argument
/// of the runtime's value-type checks: each column's `const` (built by
/// `column_const`) through the column bound the method already has
/// (`FilterColumn::ACCEPTS`, `AssignColumn::TYPE`), so the check adds no
/// bound, nor an error of its own for a column of an unsupported SQL type.
fn column_types(
    parsed: &crate::parse::columns::Parsed<'_>,
    column_const: impl Fn(&syn::Ident) -> proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let (domain_fields, diesel_idents) = diesel_arms(parsed);
    let consts = diesel_idents.iter().map(|column| column_const(column));

    quote! { &[#((#domain_fields, #consts)),*] }
}

/// One `where` predicate per column, built by `bound`, spanned to the
/// column's entity field: a column whose SQL type takes no domain value is
/// reported there (at the definition for the changeset, whose predicates name
/// no generic parameter, else where the method is called), naming the column.
fn column_bounds(
    diesel_idents: &[&syn::Ident],
    bound: impl Fn(&syn::Ident) -> proc_macro2::TokenStream,
) -> Vec<proc_macro2::TokenStream> {
    diesel_idents
        .iter()
        .map(|column| crate::utils::respanned(&bound(column), column.span()))
        .collect()
}

/// The Diesel methods of an entity with `#[diesel(table_name = table)]`: the
/// model's conditions and sort keys as Diesel's typed expressions on the
/// table's columns, one `match` arm per column, and its changeset.
///
/// They name the runtime's `diesel` module, so `code_gen` lets them through
/// the runtime's `if_diesel!`, which keeps them only with its `diesel`
/// feature.
fn diesel_items(
    parsed: &crate::parse::columns::Parsed<'_>,
    table: &syn::Path,
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    let crate::parse::columns::Parsed {
        ident, generics, ..
    } = parsed;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let (domain_fields, diesel_idents) = diesel_arms(parsed);
    let diesel = quote! { #runtime::diesel };
    let filter_bounds = column_bounds(&diesel_idents, |column| {
        quote! { #table::#column: #diesel::FilterColumn<'q, #table::table, DB> }
    });
    let order_bounds = column_bounds(&diesel_idents, |column| {
        quote! { #table::#column: #diesel::OrderColumn<'q, #table::table, DB> }
    });
    let entity = diesel_entity(
        parsed,
        table,
        runtime,
        &[filter_bounds.as_slice(), order_bounds.as_slice()].concat(),
    );
    let changeset = diesel_changeset(parsed, table, runtime);
    let column_types = column_types(parsed, |column| {
        quote! { <#table::#column as #diesel::FilterColumn<'q, #table::table, DB>>::ACCEPTS }
    });

    quote! {
        // The model's data as Diesel expressions on this entity's table, for
        // the boxed queries and changesets of Diesel's own DSL.
        impl #impl_generics #ident #type_generics #where_clause {
            /// The conditions of the model `M` (a generated filter-options
            /// model's `conditions()`), joined by `junction`, as one condition
            /// on this entity's table for a boxed query's `filter`; `None` when
            /// there are none.
            ///
            /// Fails to build unless every field of `M` has a column on the
            /// table whose SQL type takes the field's value, and unless every
            /// column's SQL type takes a domain value (mark a column storing
            /// no domain field `#[columns(skip)]`).
            ///
            /// # Errors
            ///
            /// The `ValueMismatch` of a value that does not fit its column's
            /// SQL type (only for a model without value types: one
            /// implementing `PersistedFields` by hand).
            pub fn diesel_conditions<'q, DB, M>(
                conditions: #runtime::Conditions<M>,
                junction: #runtime::Junction,
            ) -> ::core::result::Result<
                ::core::option::Option<#diesel::BoxedCondition<'q, #table::table, DB>>,
                #diesel::ValueMismatch,
            >
            where
                DB: #diesel::JoinConditions<'q, #table::table>,
                M: #runtime::PersistedFields,
                #(#filter_bounds,)*
            {
                const { #runtime::__private::assert_model_columns::<Self, M>() };
                const {
                    #runtime::__private::assert_condition_types::<Self, M>(#column_types)
                };

                let conditions = conditions
                    .into_iter()
                    .map(|filter_condition| match filter_condition.field {
                        #(#domain_fields => #diesel::FilterColumn::condition(
                            #table::#diesel_idents,
                            filter_condition,
                        ),)*
                        // Every field of `M` has a column (checked above).
                        field => ::core::result::Result::Err(#diesel::ValueMismatch { field }),
                    })
                    .collect::<::core::result::Result<::std::vec::Vec<_>, _>>()?;
                ::core::result::Result::Ok(
                    <DB as #diesel::JoinConditions<'q, #table::table>>::join(conditions, junction),
                )
            }

            /// The sort keys of the model `M` (a generated query model's
            /// `sort_keys()`), in order, as orderings of this entity's table
            /// for a boxed query's `then_order_by`.
            ///
            /// Fails to build unless every field of `M` has a column on the
            /// table.
            pub fn diesel_order_by<'q, DB, M>(
                sort_keys: #runtime::SortKeys<M>,
            ) -> ::std::vec::Vec<#diesel::BoxedOrdering<'q, #table::table, DB>>
            where
                M: #runtime::PersistedFields,
                #(#order_bounds,)*
            {
                const { #runtime::__private::assert_model_columns::<Self, M>() };

                sort_keys
                    .into_iter()
                    .filter_map(|sort_key| match sort_key.field {
                        #(#domain_fields => ::core::option::Option::Some(
                            #diesel::OrderColumn::ordering(#table::#diesel_idents, sort_key),
                        ),)*
                        // Every field of `M` has a column (checked above).
                        _ => ::core::option::Option::None,
                    })
                    .collect()
            }

            #changeset
        }

        #entity
    }
}

/// The runtime's `diesel::Entity` for a Diesel entity: its conditions and
/// sort keys again, as a trait the code generated for the wrappers names
/// (they know the entity, not its table), under the same column bounds.
fn diesel_entity(
    parsed: &crate::parse::columns::Parsed<'_>,
    table: &syn::Path,
    runtime: &syn::Path,
    column_bounds: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let crate::parse::columns::Parsed {
        ident, generics, ..
    } = parsed;
    let mut entity_generics = syn::Generics::clone(generics);
    entity_generics.params.insert(0, syn::parse_quote!('q));
    entity_generics.params.push(syn::parse_quote!(DB));
    let (impl_generics, _, _) = entity_generics.split_for_impl();
    let (_, type_generics, where_clause) = generics.split_for_impl();
    let predicates = where_clause
        .into_iter()
        .flat_map(|where_clause| &where_clause.predicates);
    let diesel = quote! { #runtime::diesel };

    quote! {
        impl #impl_generics #diesel::Entity<'q, DB> for #ident #type_generics
        where
            #(#predicates,)*
            DB: #diesel::JoinConditions<'q, #table::table>,
            #(#column_bounds,)*
        {
            type Table = #table::table;

            fn conditions<M: #runtime::PersistedFields>(
                conditions: #runtime::Conditions<M>,
                junction: #runtime::Junction,
            ) -> ::core::result::Result<
                ::core::option::Option<#diesel::BoxedCondition<'q, #table::table, DB>>,
                #diesel::ValueMismatch,
            > {
                Self::diesel_conditions(conditions, junction)
            }

            fn order_by<M: #runtime::PersistedFields>(
                sort_keys: #runtime::SortKeys<M>,
            ) -> ::std::vec::Vec<#diesel::BoxedOrdering<'q, #table::table, DB>> {
                Self::diesel_order_by(sort_keys)
            }
        }
    }
}

/// The changeset method of a Diesel entity: one `Option` slot per column,
/// filled by its index; through the runtime's `if_merge_patch!`, since it
/// names the runtime's merge-patch items.
fn diesel_changeset(
    parsed: &crate::parse::columns::Parsed<'_>,
    table: &syn::Path,
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    let (domain_fields, diesel_idents) = diesel_arms(parsed);
    let indices = (0..diesel_idents.len()).map(syn::Index::from);
    let diesel = quote! { #runtime::diesel };
    let column_types = column_types(parsed, |column| {
        quote! { <#table::#column as #diesel::AssignColumn>::TYPE }
    });
    let assign_bounds = column_bounds(&diesel_idents, |column| {
        quote! { #table::#column: #diesel::AssignColumn }
    });

    quote! {
        #runtime::__private::if_merge_patch! {
        /// The `MergePatch` model `M`'s changed values (its `changed_values`)
        /// as a Diesel changeset of this entity's table: one `Option` per
        /// column, `None` for the unchanged ones, for
        /// `diesel::update(..).set(..)`.
        ///
        /// Fails to build unless every field of `M` has a column on the table
        /// whose SQL type takes the field's value (and is nullable for a
        /// field that may be `None`), and unless every column's SQL type takes
        /// a domain value (mark a column storing no domain field
        /// `#[columns(skip)]`).
        ///
        /// # Errors
        ///
        /// The `ValueMismatch` of a value that does not fit its column's SQL
        /// type, or a `NULL` for a `NOT NULL` column (only for a model without
        /// value types: one implementing `PersistedFields` by hand).
        pub fn diesel_changeset<M>(
            assignments: #runtime::FieldValues<M>,
        ) -> ::core::result::Result<
            (#(::core::option::Option<#diesel::Assignment<#table::#diesel_idents>>,)*),
            #diesel::ValueMismatch,
        >
        where
            M: #runtime::PersistedFields,
            #(#assign_bounds,)*
        {
            const { #runtime::__private::assert_model_columns::<Self, M>() };
            const { #runtime::__private::assert_assignment_types::<Self, M>(#column_types) };

            let mut changeset = (#(::core::option::Option::<
                #diesel::Assignment<#table::#diesel_idents>,
            >::None,)*);
            for field_value in assignments {
                match field_value.field {
                    #(#domain_fields => {
                        changeset.#indices = ::core::option::Option::Some(
                            #diesel::AssignColumn::assignment(#table::#diesel_idents, field_value)?,
                        );
                    },)*
                    // Every field of `M` has a column (checked above).
                    field => return ::core::result::Result::Err(#diesel::ValueMismatch { field }),
                }
            }
            ::core::result::Result::Ok(changeset)
        }
        }
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

    // A Diesel entity: the same, plus its Diesel methods behind the runtime's
    // gate, one arm per column (`id` skipped).
    #[test]
    fn test_code_gen_diesel() {
        let syn::DeriveInput {
            ident,
            generics,
            data,
            attrs,
            ..
        } = syn::parse2(quote::quote! {
            #[derive(Queryable, Selectable, Columns)]
            #[diesel(table_name = crate::schema::orders)]
            #[columns(model = domain::models::order::OrderModel)]
            struct Order {
                #[columns(skip)]
                id: Uuid,
                #[domain_field(delivery_date)]
                delivered_at: DateTime<Utc>,
                #[diesel(column_name = order_number)]
                number: String,
            }
        })
        .unwrap();
        let input = crate::parse::columns::Input::new(ident, generics, data, attrs);
        let mapping_paths = crate::mapping::columns::MappingPaths {
            domain_model: syn::parse_str("domain::models::order::OrderModel").unwrap(),
            runtime: syn::parse_quote!(::queryweave_sql),
        };

        let parsed = crate::parse::columns::parse(&input).unwrap();
        let table = parsed.diesel_table.clone().unwrap();

        // Behind the runtime's gate...
        let gated = super::code_gen(&parsed, mapping_paths).to_string();
        assert!(gated.contains(":: queryweave_sql :: __private :: if_diesel !"));

        // ...the Diesel methods.
        let items = super::diesel_items(&parsed, &table, &syn::parse_quote!(::queryweave_sql));
        insta::assert_snapshot!(prettyplease::unparse(
            &syn::parse2::<syn::File>(items).unwrap()
        ));
    }
}
