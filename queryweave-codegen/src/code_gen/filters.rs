//! The `Filters` code: a root model's, or a mirror's.

mod mirror;
mod root;

use quote::{quote, quote_spanned};

/// The docs of the generated filter fields (a root's filter options, a
/// mirror's query parameters and filter options), one per field: which field
/// each filters, and how.
fn filter_field_docs(
    field_columns: &[String],
    field_operators: &[crate::analyze::filters::FilterOperator],
) -> Vec<String> {
    field_columns
        .iter()
        .zip(field_operators)
        .map(|(field, operator)| {
            format!(
                " Keeps the items whose `{field}` {} the given value.",
                operator.phrase()
            )
        })
        .collect()
}

/// The docs of the generated sort-field variants, one per variant.
fn sort_variant_docs(snake_cased_sort_fields: &[syn::Ident]) -> Vec<String> {
    snake_cased_sort_fields
        .iter()
        .map(|field| format!(" Sort by `{field}`."))
        .collect()
}

/// What a struct (root model or mirror) publishes for the structs mirroring
/// it: its `Filterable` impl, and the `From` impls from the types mirroring it,
/// which implement the `queryweave` role traits (a mirror cannot implement
/// `From` for a projection of `Filterable` itself; see `queryweave::Filterable`).
struct Filterable<'a> {
    /// The generated types (`query` being a root's query model, or a mirror's
    /// query parameters).
    idents: &'a crate::conventions::FiltersNames,
    query: &'a syn::Ident,
    /// `Filterable::Root`.
    root: proc_macro2::TokenStream,
    /// The pattern binding `Filterable::query`'s `sort`: `sort`, or `_` for a
    /// query without one.
    sort_binding: proc_macro2::TokenStream,
    /// `Filterable::query`'s body, from `page`, `page_size`, `sort` (as bound
    /// by `sort_binding`) and `filters`.
    query_body: proc_macro2::TokenStream,
    /// `Filterable::into_root_query`'s body, from `query`.
    into_root_body: proc_macro2::TokenStream,
    /// Where the code naming the root is reported: the call site for a root
    /// model, the mirrored model's path for a mirror (whose root comes from
    /// it, so a model that does not derive `Filters` is reported there).
    root_span: proc_macro2::Span,
}

fn quote_filterable_items(
    filterable: Filterable<'_>,
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    let Filterable {
        idents,
        query,
        root,
        sort_binding,
        query_body,
        into_root_body,
        root_span,
    } = filterable;
    let crate::conventions::FiltersNames {
        filters,
        sort_field,
        sort_details,
        input,
        ..
    } = idents;
    let root_runtime = crate::utils::respanned(runtime, root_span);
    let into_root_query = quote_spanned! {root_span=>
        fn into_root_query(query: #query) -> <Self::Root as #root_runtime::Filterable>::Query {
            #into_root_body
        }
    };
    // A trait impl generic over `#input` rather than an inherent method: its
    // signature names `#input`, which may be private while the generated
    // types are public, and an impl naming it is no more visible than it.
    let into_root_query_impl = quote_spanned! {root_span=>
        impl #root_runtime::IntoRootQuery<#input> for #query {
            fn into_root_query(
                self,
            ) -> <<#input as #root_runtime::Filterable>::Root as #root_runtime::Filterable>::Query {
                <#input as #root_runtime::Filterable>::into_root_query(self)
            }
        }
    };

    quote! {
        /// The types generated for this struct, for code mirroring it (e.g. an
        /// API DTO) to name through it.
        impl #runtime::Filterable for #input {
            type Root = #root;
            type Query = #query;
            type FilterOptions = #filters;
            type SortField = #sort_field;
            type SortDetails = #sort_details;

            fn query(
                page: Option<u32>,
                page_size: Option<u32>,
                #sort_binding: Option<Vec<#sort_details>>,
                filters: #filters,
            ) -> #query {
                #query_body
            }

            #into_root_query
        }

        #into_root_query_impl

        impl<T: #runtime::QueryOf<#input>> From<T> for #query {
            fn from(value: T) -> Self {
                #runtime::QueryOf::into_query(value)
            }
        }

        impl<T: #runtime::FilterOptionsOf<#input>> From<T> for #filters {
            fn from(value: T) -> Self {
                #runtime::FilterOptionsOf::into_filter_options(value)
            }
        }

        impl<T: #runtime::SortFieldOf<#input>> From<T> for #sort_field {
            fn from(value: T) -> Self {
                #runtime::SortFieldOf::into_sort_field(value)
            }
        }

        impl<T: #runtime::SortDetailsOf<#input>> From<T> for #sort_details {
            fn from(value: T) -> Self {
                #runtime::SortDetailsOf::into_sort_details(value)
            }
        }
    }
}

/// Generate the `Filters` code for a struct of the given role.
#[must_use]
pub fn code_gen(
    mut ir: crate::lower::filters::IntermediaryRepresentation,
    mapping_paths: &crate::mapping::filters::MappingPaths,
) -> proc_macro2::TokenStream {
    // The names the mapping step settled on: the conventions', with the
    // struct's renames.
    ir.idents = mapping_paths.names.clone();
    let runtime = &mapping_paths.runtime;
    match &mapping_paths.role {
        crate::mapping::filters::Role::Root => root::code_gen(ir, runtime),
        crate::mapping::filters::Role::Mirror {
            model,
            integrations,
        } => mirror::code_gen(ir, model, integrations, runtime),
    }
}

#[cfg(test)]
mod tests {
    use crate::conventions::{Conventions, DefaultConventions};
    use crate::mapping::filters::{MappingPaths, Role};

    fn intermediary_representation() -> crate::lower::filters::IntermediaryRepresentation {
        crate::lower::filters::IntermediaryRepresentation {
            field_idents: vec![quote::format_ident!("username_sw")],
            equivalent_field_idents: vec![quote::format_ident!("username_sw")],
            field_value_types: vec![syn::parse_str::<syn::Type>("String").unwrap()],
            field_columns: vec![String::from("username")],
            field_operators: vec![crate::analyze::filters::FilterOperator::StartsWith],
            idents: DefaultConventions::filters_names(&quote::format_ident!("UserModel")).unwrap(),
            sort: Some(crate::lower::filters::Sort {
                variants: vec![quote::format_ident!("Username")],
                fields: vec![quote::format_ident!("username")],
                equivalent_variants: vec![quote::format_ident!("Username")],
            }),
        }
    }

    // Every runtime path, in either role, is the one the mapping step chose
    // (the conventions' default, or `#[queryweave(crate = path)]`).
    #[test]
    fn test_code_gen_names_the_runtime_crate_of_the_mapping_paths() {
        let root = super::code_gen(
            intermediary_representation(),
            &MappingPaths {
                role: Role::Root,
                runtime: syn::parse_quote!(my::core),
                names: intermediary_representation().idents,
            },
        )
        .to_string();
        let mirror = super::code_gen(
            intermediary_representation(),
            &MappingPaths {
                role: Role::Mirror {
                    model: syn::parse_quote!(domain::UserModel),
                    integrations: vec![crate::integrations::Active {
                        integration: crate::integrations::BUILT_IN[0],
                        flag: quote::format_ident!("serde"),
                    }],
                },
                runtime: syn::parse_quote!(my::api),
                names: intermediary_representation().idents,
            },
        )
        .to_string();

        assert!(root.contains("my :: core :: Filterable"));
        assert!(mirror.contains("my :: api :: QueryOf"));
        assert!(mirror.contains("my :: api :: PageDto"));
        for code in [&root, &mirror] {
            assert!(!code.contains("queryweave"), "{code}");
        }
    }
}
