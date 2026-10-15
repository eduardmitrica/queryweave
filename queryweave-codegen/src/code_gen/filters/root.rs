use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// The runtime `queryweave::FilterOperator` variant a filter field compares with.
fn quote_operator(
    operator: crate::analyze::filters::FilterOperator,
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    use crate::analyze::filters::FilterOperator;

    match operator {
        FilterOperator::LessThan => quote!(#runtime::FilterOperator::LessThan),
        FilterOperator::Equal | FilterOperator::Boolean => {
            quote!(#runtime::FilterOperator::Equal)
        },
        FilterOperator::GreaterThan => quote!(#runtime::FilterOperator::GreaterThan),
        FilterOperator::StartsWith => quote!(#runtime::FilterOperator::StartsWith),
        FilterOperator::Contains => quote!(#runtime::FilterOperator::Contains),
        FilterOperator::EndsWith => quote!(#runtime::FilterOperator::EndsWith),
    }
}

/// The assertion (see `quote_checks`) that the raw value has the
/// `queryweave` filter kind an operator requires, if any (`eq` applies to any
/// raw value).
fn quote_operator_kind(operator: crate::analyze::filters::FilterOperator) -> Option<syn::Ident> {
    use crate::analyze::filters::FilterOperator;

    let assertion = match operator {
        FilterOperator::LessThan | FilterOperator::GreaterThan => "_assert_ordered",
        FilterOperator::StartsWith | FilterOperator::Contains | FilterOperator::EndsWith => {
            "_assert_text"
        },
        FilterOperator::Boolean => "_assert_boolean",
        FilterOperator::Equal => return None,
    };

    Some(quote::format_ident!("{assertion}"))
}

/// Compile-time checks, spanned to each filtered field's type, that its raw
/// value converts into a `queryweave::Value` and supports the operators
/// placed on it. The compiler
/// resolves the actual type (aliases, value objects, ...) instead of the macro
/// guessing from the type's name.
fn quote_checks(
    field_value_types: &[syn::Type],
    field_operators: &[crate::analyze::filters::FilterOperator],
    runtime: &syn::Path,
) -> Vec<proc_macro2::TokenStream> {
    let mut seen = std::collections::HashSet::new();

    field_value_types
        .iter()
        .zip(field_operators)
        .flat_map(|(value_type, operator)| {
            // Every token of a check carries the field type's span: with any
            // call-site token left in the expression, rustc reports the whole
            // call at the derive instead of at the field.
            let span = value_type.span();
            let runtime = crate::utils::respanned(runtime, span);
            let raw = quote_spanned!(span=> <#value_type as #runtime::RawValue>::Raw);
            let type_key = quote!(#value_type).to_string();

            let value = seen
                .insert((type_key.clone(), String::from("value")))
                .then(|| quote_spanned!(span=> _assert_value::<#raw>();));
            let kind = quote_operator_kind(*operator).and_then(|assertion| {
                let assertion = syn::Ident::new(&assertion.to_string(), span);
                seen.insert((type_key, assertion.to_string()))
                    .then(|| quote_spanned!(span=> #assertion::<#raw>();))
            });

            value.into_iter().chain(kind)
        })
        .collect()
}

/// One statement per filter field collecting it, when set, as a
/// `queryweave::FilterCondition` against the column and operator decided at
/// expansion time. What the condition means in a query is up to the
/// persistence layer.
fn quote_condition_statements(
    field_idents: &[syn::Ident],
    field_columns: &[String],
    field_operators: &[crate::analyze::filters::FilterOperator],
    runtime: &syn::Path,
) -> Vec<proc_macro2::TokenStream> {
    field_idents
        .iter()
        .zip(field_columns)
        .zip(field_operators)
        .map(|((field_ident, column), operator)| {
            let operator = quote_operator(*operator, runtime);

            quote! {
                if let Some(value) = &self.#field_ident {
                    conditions.push(#runtime::FilterCondition {
                        field: #column,
                        operator: #operator,
                        value: #runtime::IntoValue::into_value(::core::clone::Clone::clone(value)),
                    });
                }
            }
        })
        .collect()
}

/// The sorting items: the sort-field enum (with `field_name`) and the sort
/// details struct, then the query model's `sort_keys`, handing the requested
/// sorting to the persistence layer as data. Without `#[sortable]` fields the
/// enum is simply empty; the `match` in `field_name` is then exhaustive with
/// no arms.
fn quote_sort_items(
    idents: &crate::conventions::FiltersNames,
    sort: Option<crate::lower::filters::Sort>,
    runtime: &syn::Path,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let crate::conventions::FiltersNames {
        query,
        sort_field,
        sort_details,
        ..
    } = idents;
    // The domain model is the target of any `#[equivalent_field]` mapping, so
    // its own names are the ones that matter here.
    let crate::lower::filters::Sort {
        variants: sort_fields,
        fields: snake_cased_sort_fields,
        ..
    } = sort.unwrap_or(crate::lower::filters::Sort {
        variants: Vec::new(),
        fields: Vec::new(),
        equivalent_variants: Vec::new(),
    });

    let variant_docs = super::sort_variant_docs(&snake_cased_sort_fields);

    let sort_types = quote! {
        /// Sortable fields enum.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #sort_field {
            #(#[doc = #variant_docs] #sort_fields),*
        }

        impl #sort_field {
            /// The name of the domain field sorted by.
            #[must_use]
            pub const fn field_name(self) -> &'static str {
                match self {
                    #(Self::#sort_fields => stringify!(#snake_cased_sort_fields)),*
                }
            }
        }

        /// Struct containing information for sorting
        /// such as the field by which we sort and
        /// the order in which we want to sort it.
        #[derive(Debug, Clone)]
        pub struct #sort_details {
            /// The field to sort by.
            pub field: #sort_field,

            /// The direction to sort in.
            pub order: #runtime::SortDirectionModel
        }
    };

    let sort_keys = quote! {
        impl #query {
            /// The requested sorting, in order, as database-agnostic sort keys
            /// for the persistence layer to turn into a query.
            #[must_use]
            pub fn sort_keys(&self) -> #runtime::SortKeys<Self> {
                #runtime::__private::model_data(
                    self.sort
                        .iter()
                        .flatten()
                        .map(|details| #runtime::SortKey {
                            field: details.field.field_name(),
                            direction: details.order,
                        })
                        .collect(),
                )
            }
        }
    };

    (sort_types, sort_keys)
}

/// The `queryweave::Filterable` impl of a root model: its own root, building
/// its query model.
fn quote_filterable_items(
    idents: &crate::conventions::FiltersNames,
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    let query = &idents.query;

    super::quote_filterable_items(
        super::Filterable {
            idents,
            query,
            root: quote!(Self),
            sort_binding: quote!(sort),
            query_body: quote! {
                #query {
                    page,
                    page_size,
                    sort,
                    and: filters,
                }
            },
            into_root_body: quote!(query),
            root_span: proc_macro2::Span::call_site(),
        },
        runtime,
    )
}

/// Generate the `Filters` code of a root model.
#[must_use]
pub fn code_gen(
    ir: crate::lower::filters::IntermediaryRepresentation,
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    // The domain model is the target of any `#[equivalent_field]` mapping,
    // so the equivalent names are not used here.
    let crate::lower::filters::IntermediaryRepresentation {
        idents,
        field_idents,
        field_value_types,
        field_columns,
        field_operators,
        sort,
        ..
    } = ir;
    let crate::conventions::FiltersNames {
        filters,
        query,
        sort_details,
        ..
    } = &idents;

    // The domain fields each model hands to the persistence layer, once
    // each, in declaration order: the filtered ones, plus the sortable ones
    // for the query model. The persistence layer checks at compile time
    // that each has a column.
    let mut filter_field_names = Vec::new();
    let mut filter_field_value_types = Vec::new();
    for (column, value_type) in field_columns.iter().zip(&field_value_types) {
        if !filter_field_names.contains(column) {
            filter_field_names.push(column.clone());
            filter_field_value_types.push(value_type);
        }
    }
    // The value type of each, for the persistence layer to check against its
    // column's type: the filter fields of one domain field share it. Spanned
    // to the model field, like the other uses of its type.
    let filter_field_types = filter_field_value_types.iter().map(|value_type| {
        let runtime = crate::utils::respanned(runtime, value_type.span());
        quote_spanned! {value_type.span()=>
            <<#value_type as #runtime::RawValue>::Raw as #runtime::IntoValue>::TYPE
        }
    });
    let mut query_field_names = filter_field_names.clone();
    for sort_field in sort.iter().flat_map(|sort| &sort.fields) {
        let name = sort_field.to_string();
        if !query_field_names.contains(&name) {
            query_field_names.push(name);
        }
    }

    let field_docs = super::filter_field_docs(&field_columns, &field_operators);
    let (sort_types, sort_keys) = quote_sort_items(&idents, sort, runtime);
    let filterable_items = quote_filterable_items(&idents, runtime);

    let condition_statements =
        quote_condition_statements(&field_idents, &field_columns, &field_operators, runtime);
    // A sortable-only model has no filters; `let mut` would then be unused.
    let conditions_body = if condition_statements.is_empty() {
        quote!(Vec::new())
    } else {
        quote! {
            let mut conditions = Vec::new();
            #(#condition_statements)*
            conditions
        }
    };
    let checks = quote_checks(&field_value_types, &field_operators, runtime);
    // Spanned to the model field, so an error about the type (e.g. no
    // `RawValue`), including those surfacing through the derives on the
    // filters struct, is reported on the field.
    let filter_fields = field_value_types.iter().map(|value_type| {
        let runtime = crate::utils::respanned(runtime, value_type.span());
        quote_spanned!(value_type.span()=> Option<<#value_type as #runtime::RawValue>::Raw>)
    });

    quote! {
        #sort_types

        /// Filters derived from the original struct's fields.
        #[derive(Debug, Default, Clone)]
        pub struct #filters {
            #(#[doc = #field_docs] pub #field_idents: #filter_fields),*
        }

        /// The domain fields these filters compare, and their value types,
        /// for the persistence layer to map onto columns.
        impl #runtime::PersistedFields for #filters {
            const FIELD_NAMES: &'static [&'static str] = &[#(#filter_field_names),*];
            const FIELD_TYPES: &'static [#runtime::ValueType] = &[#(#filter_field_types),*];
        }

        impl #filters {
            /// Check if there are any filters that are not `None`.
            #[must_use]
            pub fn has_options(&self) -> bool {
                false #(|| self.#field_idents.is_some())*
            }

            /// The filters that are set, as database-agnostic conditions for
            /// the persistence layer to turn into a query.
            #[must_use]
            pub fn conditions(&self) -> #runtime::Conditions<Self> {
                #runtime::__private::model_data({ #conditions_body })
            }
        }

        /// The struct used for keeping and page, page_size, sort, order and the inner filters.
        #[derive(Debug, Default, Clone)]
        pub struct #query {
            /// Page to select (0 based).
            pub page: Option<u32>,
            /// Number of elements per page to get back.
            pub page_size: Option<u32>,
            /// Sort by some sortable fields.
            pub sort: Option<Vec<#sort_details>>,
            /// AND filter options.
            pub and: #filters,
        }

        #sort_keys

        /// The domain fields this query filters or sorts by, for the
        /// persistence layer to map onto columns.
        impl #runtime::PersistedFields for #query {
            const FIELD_NAMES: &'static [&'static str] = &[#(#query_field_names),*];
        }

        #filterable_items

        // Never called: an anonymous const, so the checks add no name to
        // the user's types.
        const _: fn() = || {
            fn _assert_value<T: #runtime::IntoValue>() {}
            fn _assert_ordered<T: #runtime::Ordered>() {}
            fn _assert_text<T: #runtime::Text>() {}
            fn _assert_boolean<T: #runtime::Boolean>() {}
            #(#checks)*
        };
    }
}

#[cfg(test)]
mod tests {

    /// Pretty-print the generated items, which also asserts they form a valid
    /// Rust file.
    fn pretty(token_stream: proc_macro2::TokenStream) -> String {
        prettyplease::unparse(&syn::parse2::<syn::File>(token_stream).unwrap())
    }

    #[test]
    fn test_code_gen_for_sortable() {
        let idents = crate::conventions::FiltersNames {
            query: quote::format_ident!("UsersQueryModel"),
            filters: quote::format_ident!("UsersFilterOptionsModel"),
            sort_field: quote::format_ident!("UserSortFieldModel"),
            sort_details: quote::format_ident!("UserSortDetailsModel"),
            input: quote::format_ident!("UserModel"),
            page: quote::format_ident!("UsersPageModel"),
            query_params: quote::format_ident!("UsersQueryParams"),
        };
        let intermediary_representation = crate::lower::filters::IntermediaryRepresentation {
            field_idents: vec![quote::format_ident!("username_sw")],
            equivalent_field_idents: vec![quote::format_ident!("username_sw")],
            field_value_types: vec![syn::parse_str::<syn::Type>("String").unwrap()],
            field_columns: vec![String::from("username")],
            field_operators: vec![crate::analyze::filters::FilterOperator::StartsWith],
            idents,
            sort: Some(crate::lower::filters::Sort {
                variants: vec![quote::format_ident!("Username")],
                fields: vec![quote::format_ident!("username")],
                equivalent_variants: vec![quote::format_ident!("Username")],
            }),
        };

        insta::assert_snapshot!(pretty(super::code_gen(
            intermediary_representation,
            &syn::parse_quote!(::queryweave)
        )));
    }

    #[test]
    fn test_code_gen_for_unsortable() {
        let idents = crate::conventions::FiltersNames {
            query: quote::format_ident!("UsersQueryModel"),
            filters: quote::format_ident!("UsersFilterOptionsModel"),
            sort_field: quote::format_ident!("UserSortFieldModel"),
            sort_details: quote::format_ident!("UserSortDetailsModel"),
            input: quote::format_ident!("UserModel"),
            page: quote::format_ident!("UsersPageModel"),
            query_params: quote::format_ident!("UsersQueryParams"),
        };
        let intermediary_representation = crate::lower::filters::IntermediaryRepresentation {
            field_idents: vec![quote::format_ident!("username_sw")],
            equivalent_field_idents: vec![quote::format_ident!("username_sw")],
            field_value_types: vec![syn::parse_str::<syn::Type>("String").unwrap()],
            field_columns: vec![String::from("username")],
            field_operators: vec![crate::analyze::filters::FilterOperator::StartsWith],
            idents,
            sort: None,
        };

        insta::assert_snapshot!(pretty(super::code_gen(
            intermediary_representation,
            &syn::parse_quote!(::queryweave)
        )));
    }
}
