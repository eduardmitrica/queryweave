use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// The items that only exist for a DTO with `#[sortable]` fields: the
/// conversions between the DTO and domain sort types, and the integrations'
/// own (e.g. validator's `validate_sort`). Each DTO variant maps onto the
/// domain variant of its equivalent field (see `#[equivalent_field(name)]`), spanned to that name, so one the
/// domain enum lacks is reported there.
///
/// The domain types are named through `queryweave::Filterable` on the model
/// (aliased, since a qualified path cannot name a struct literal or a
/// variant), and the conversions are the `queryweave` role traits: the
/// model's derive turns them into `From` impls (see `queryweave::Filterable`).
fn quote_sortable_items(
    idents: &crate::conventions::FiltersNames,
    model: &syn::Path,
    sort_fields: &[syn::Ident],
    equivalent_sort_fields: &[syn::Ident],
    integration_items: &proc_macro2::TokenStream,
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    // Spanned to the model's path like the conversions naming it (see below).
    let runtime = crate::utils::respanned(runtime, model.span());
    let sort_field = &idents.sort_field;
    let sort_details = &idents.sort_details;
    // The match arms keep the call-site span on the paths, so a variant the
    // domain enum lacks is underlined at its name alone (rustc would otherwise
    // join the model's path and the name into one span).
    let into_arms = sort_fields
        .iter()
        .zip(equivalent_sort_fields)
        .map(|(field, equivalent)| quote!(Self::#field => __SortField::#equivalent));
    let from_arms = sort_fields
        .iter()
        .zip(equivalent_sort_fields)
        .map(|(field, equivalent)| quote!(__SortField::#equivalent => Self::#field));
    // The order converts into the mirrored model's: between `SortDirectionDto`
    // and `SortDirectionModel`, or into itself. Not the user's
    // code, so spanned to the macro (lints treat it as generated, e.g. an
    // identity conversion).
    let order_conversion = quote!(self.order.into());
    // Spanned to the model's path, so a model that does not derive `Filters`
    // is reported there rather than at the derive.
    let conversions = quote_spanned! {model.span()=>
        const _: () = {
            type __SortField = <#model as #runtime::Filterable>::SortField;
            type __SortDetails = <#model as #runtime::Filterable>::SortDetails;

            impl #runtime::SortFieldOf<#model> for #sort_field {
                fn into_sort_field(self) -> __SortField {
                    match self {
                        #(#into_arms),*
                    }
                }

                fn from_sort_field(field: __SortField) -> Self {
                    match field {
                        #(#from_arms),*
                    }
                }
            }

            impl #runtime::SortDetailsOf<#model> for &#sort_details {
                fn into_sort_details(self) -> __SortDetails {
                    __SortDetails {
                        field: #runtime::SortFieldOf::<#model>::into_sort_field(self.field),
                        order: #order_conversion
                    }
                }
            }
        };
    };

    quote! {
        #conversions

        #integration_items
    }
}

/// The conversions from the query params into the model's query and filter
/// options models.
fn quote_query_conversions(
    model: &syn::Path,
    query_params: &syn::Ident,
    sort_conversion: &proc_macro2::TokenStream,
    field_idents: &[syn::Ident],
    equivalent_field_idents: &[syn::Ident],
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    // Spanned to the model's path like the conversions naming it (see below).
    let runtime = crate::utils::respanned(runtime, model.span());
    // Each query parameter fills the domain filter of its equivalent field
    // (see `#[equivalent_field(name)]`), spanned to that name, so one the
    // domain filters lack is reported there. The literal keeps the call-site
    // span, so rustc reports the name alone.
    let filter_options_literal = quote! {
        __FilterOptions {
            #(#equivalent_field_idents: self.#field_idents),*
        }
    };

    // The domain types are named through `queryweave::Filterable` on the
    // model, and the conversions are the `queryweave` role traits, which
    // the model's derive turns into `From` impls. Spanned to the model's
    // path, so a model that does not derive `Filters` is reported there
    // rather than at the derive.
    quote_spanned! {model.span()=>
        const _: () = {
            type __Query = <#model as #runtime::Filterable>::Query;
            type __FilterOptions = <#model as #runtime::Filterable>::FilterOptions;

            // Through `Filterable::query`, so the model is a root model or
            // another mirror alike.
            impl #runtime::QueryOf<#model> for #query_params {
                fn into_query(self) -> __Query {
                    let sort = #sort_conversion;
                    <#model as #runtime::Filterable>::query(
                        self.page,
                        self.page_size,
                        sort,
                        #runtime::FilterOptionsOf::<#model>::into_filter_options(self),
                    )
                }
            }

            impl #runtime::FilterOptionsOf<#model> for #query_params {
                fn into_filter_options(self) -> __FilterOptions {
                    #filter_options_literal
                }
            }
        };
    }
}

/// How the query parameters handle `sort`.
struct SortParameter {
    /// The `sort` field of the query parameters, with its attributes: none
    /// for an unsortable mirror.
    field: proc_macro2::TokenStream,
    /// The `sort` parameter, converted into the mirrored model's sort details.
    conversion: proc_macro2::TokenStream,
    /// The items only a sortable mirror has (see `quote_sortable_items`).
    items: proc_macro2::TokenStream,
}

/// The `sort` parameter: converted (and given the integrations' attributes,
/// e.g. validated) for a sortable mirror. An unsortable one still gets the (empty) sort types, so
/// the mirrored model's query can be built the same way, but has no `sort`
/// field: nothing to deserialize, document or skip, whichever integrations
/// it uses.
fn quote_sort_parameter(
    idents: &crate::conventions::FiltersNames,
    model: &syn::Path,
    sort_fields: &[syn::Ident],
    equivalent_sort_fields: &[syn::Ident],
    integrated: &Integrated,
    runtime: &syn::Path,
) -> SortParameter {
    if sort_fields.is_empty() {
        return SortParameter {
            field: quote!(),
            conversion: quote!(None),
            items: quote!(),
        };
    }

    let sort_details = &idents.sort_details;
    let attributes = &integrated.sort;

    SortParameter {
        field: quote! {
            /// Sort by some sortable fields.
            #attributes
            pub sort: Option<Vec<#sort_details>>,
        },
        conversion: quote!(
            self.sort
                .as_ref()
                .map(|sort| sort.into_iter().map(Into::into).collect())
        ),
        items: quote_sortable_items(
            idents,
            model,
            sort_fields,
            equivalent_sort_fields,
            &integrated.sortable_items,
            runtime,
        ),
    }
}

/// What a mirror publishes for the structs mirroring it in turn: its filter
/// options, its `Filterable` impl (its root being `model`'s), the `From`
/// impls and `IntoRootQuery` on its query parameters.
fn quote_mirrorable_items(
    idents: &crate::conventions::FiltersNames,
    model: &syn::Path,
    sortable: bool,
    field_idents: &[syn::Ident],
    field_value_types: &[syn::Type],
    field_docs: &[String],
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    let crate::conventions::FiltersNames {
        filters,
        query_params,
        ..
    } = idents;
    // Spanned to the model's path, so a model that does not derive `Filters`
    // is reported there rather than at the derive.
    let model_runtime = crate::utils::respanned(runtime, model.span());
    let root = quote_spanned!(model.span()=> <#model as #model_runtime::Filterable>::Root);
    let into_root_body = quote_spanned! {model.span()=>
        <#model as #model_runtime::Filterable>::into_root_query(
            #model_runtime::QueryOf::<#model>::into_query(query),
        )
    };
    // An unsortable mirror's query parameters have no `sort`, and its sort
    // details type is empty, so there is no sort to carry over.
    let (sort_binding, sort_field) = if sortable {
        (quote!(sort), quote!(sort,))
    } else {
        (quote!(_), quote!())
    };
    let filterable = super::quote_filterable_items(
        super::Filterable {
            idents,
            query: query_params,
            root,
            sort_binding,
            query_body: quote! {
                #query_params {
                    page,
                    page_size,
                    #sort_field
                    #(#field_idents: filters.#field_idents),*
                }
            },
            into_root_body,
            root_span: model.span(),
        },
        runtime,
    );

    quote! {
        /// The filters of the query parameters, for a struct mirroring this
        /// one to convert into.
        #[derive(Debug, Default, Clone)]
        pub struct #filters {
            #(#[doc = #field_docs] pub #field_idents: Option<#field_value_types>),*
        }

        #filterable

    }
}

/// The derives and attributes of the generated types: their own, then those
/// of the integrations the mirror opts into (none for a plain mirror, which
/// names only the runtime), in the order of `integrations::BUILT_IN`.
struct Integrated {
    /// The sort field enum's.
    sort_field: crate::integrations::TypeAdditions,
    /// The sort details struct's.
    sort_details: crate::integrations::TypeAdditions,
    /// The query parameters struct's.
    query_params: crate::integrations::TypeAdditions,
    /// The attributes of the `page_size` parameter.
    page_size: proc_macro2::TokenStream,
    /// The attributes of the `sort` parameter (of a sortable mirror).
    sort: proc_macro2::TokenStream,
    /// The integrations' items of any mirror.
    items: proc_macro2::TokenStream,
    /// The integrations' items of a sortable mirror.
    sortable_items: proc_macro2::TokenStream,
    /// The type of the sort direction: the integrations' (which agree on it,
    /// see the mapping step), or the core `SortDirectionModel`.
    order: proc_macro2::TokenStream,
    /// The page alias, when the integrations provide a page type (on which
    /// they agree).
    page: proc_macro2::TokenStream,
}

/// `own` derives, followed by the integrations' `additions`.
fn merged(
    own: proc_macro2::TokenStream,
    additions: impl Iterator<Item = crate::integrations::TypeAdditions>,
) -> crate::integrations::TypeAdditions {
    let mut merged = crate::integrations::TypeAdditions {
        derives: vec![own],
        attributes: proc_macro2::TokenStream::new(),
    };
    for crate::integrations::TypeAdditions {
        derives,
        attributes,
    } in additions
    {
        merged.derives.extend(derives);
        merged.attributes.extend(attributes);
    }
    merged
}

fn integrated(
    idents: &crate::conventions::FiltersNames,
    integrations: &[crate::integrations::Active],
    runtime: &syn::Path,
) -> Integrated {
    // Each integration with what it knows of the mirror (its own flag).
    let active = integrations
        .iter()
        .map(|active| (active.integration, active.context(idents, runtime)))
        .collect::<Vec<_>>();
    let page = &idents.page;

    Integrated {
        sort_field: merged(
            quote!(Debug, Clone, Copy, PartialEq, Eq, Hash),
            active
                .iter()
                .map(|(integration, context)| integration.sort_field(context)),
        ),
        sort_details: merged(
            quote!(Debug, Clone, PartialEq, Eq),
            active
                .iter()
                .map(|(integration, context)| integration.sort_details(context)),
        ),
        query_params: merged(
            quote!(Debug, Default, Clone),
            active
                .iter()
                .map(|(integration, context)| integration.query_params(context)),
        ),
        page_size: active
            .iter()
            .map(|(integration, context)| integration.page_size(context))
            .collect(),
        sort: active
            .iter()
            .map(|(integration, context)| integration.sort(context))
            .collect(),
        items: active
            .iter()
            .map(|(integration, context)| integration.items(context))
            .collect(),
        sortable_items: active
            .iter()
            .map(|(integration, context)| integration.sortable_items(context))
            .collect(),
        order: active
            .iter()
            .find_map(|(integration, context)| integration.sort_direction(context))
            .unwrap_or_else(|| quote!(#runtime::SortDirectionModel)),
        page: active
            .iter()
            .find_map(|(integration, context)| integration.page(context))
            .map(|page_type| {
                quote! {
                    /// A page of these structs.
                    pub type #page = #page_type;
                }
            })
            .unwrap_or_default(),
    }
}

/// The sort field enum (its variants documented by `variant_docs`) and the
/// sort details struct, each with the integrations' derives and attributes.
fn quote_sort_types(
    idents: &crate::conventions::FiltersNames,
    sort_fields: &[syn::Ident],
    variant_docs: &[String],
    integrated: &Integrated,
) -> proc_macro2::TokenStream {
    let crate::conventions::FiltersNames {
        sort_field,
        sort_details,
        ..
    } = idents;
    let Integrated {
        sort_field:
            crate::integrations::TypeAdditions {
                derives: sort_field_derives,
                attributes: sort_field_attributes,
            },
        sort_details:
            crate::integrations::TypeAdditions {
                derives: sort_details_derives,
                attributes: sort_details_attributes,
            },
        order,
        ..
    } = integrated;
    let sort_variants_count = sort_fields.len();

    quote! {
        /// Sortable fields enum.
        #[derive(#(#sort_field_derives),*)]
        #sort_field_attributes
        pub enum #sort_field {
            #(#[doc = #variant_docs] #sort_fields),*
        }

        impl #sort_field {
            /// The number of sortable fields, i.e. the most sort details a query may carry.
            pub const COUNT: u64 = #sort_variants_count as u64;
        }

        /// A field to sort by, and the direction to sort it in.
        #[derive(#(#sort_details_derives),*)]
        #sort_details_attributes
        pub struct #sort_details {
            /// The field to sort by.
            pub field: #sort_field,
            /// The direction to sort in.
            pub order: #order
        }
    }
}

/// Generate the `Filters` code of a struct mirroring the model `model` (a root
/// model, or another mirror), with the integrations it opts into.
#[must_use]
pub fn code_gen(
    ir: crate::lower::filters::IntermediaryRepresentation,
    model: &syn::Path,
    integrations: &[crate::integrations::Active],
    runtime: &syn::Path,
) -> proc_macro2::TokenStream {
    let crate::lower::filters::IntermediaryRepresentation {
        idents,
        field_idents,
        equivalent_field_idents,
        field_value_types,
        field_columns,
        field_operators,
        sort,
    } = ir;
    let field_docs = super::filter_field_docs(&field_columns, &field_operators);
    let query_params = &idents.query_params;
    let integrated = integrated(&idents, integrations, runtime);

    let (sort_fields, snake_cased_sort_fields, equivalent_sort_fields) = sort
        .map(|sort| (sort.variants, sort.fields, sort.equivalent_variants))
        .unwrap_or_default();
    let variant_docs = super::sort_variant_docs(&snake_cased_sort_fields);
    let sort_types = quote_sort_types(&idents, &sort_fields, &variant_docs, &integrated);
    let SortParameter {
        field: sort_parameter,
        conversion: sort_conversion,
        items: sortable_items,
    } = quote_sort_parameter(
        &idents,
        model,
        &sort_fields,
        &equivalent_sort_fields,
        &integrated,
        runtime,
    );
    let Integrated {
        query_params:
            crate::integrations::TypeAdditions {
                derives: query_params_derives,
                attributes: query_params_attributes,
            },
        page_size: page_size_attributes,
        items: integration_items,
        page,
        ..
    } = integrated;

    let conversions = quote_query_conversions(
        model,
        query_params,
        &sort_conversion,
        &field_idents,
        &equivalent_field_idents,
        runtime,
    );
    let mirrorable_items = quote_mirrorable_items(
        &idents,
        model,
        !sort_fields.is_empty(),
        &field_idents,
        &field_value_types,
        &field_docs,
        runtime,
    );

    // The integrations' own items first: their checks (e.g. that a runtime
    // feature is enabled) are then reported before what fails because of them.
    quote! {
        #integration_items

        #sort_types

        #sortable_items

        /// The struct used for keeping and page, page_size, sort, order and the inner filters.
        #[derive(#(#query_params_derives),*)]
        #query_params_attributes
        pub struct #query_params {
            /// Page to select (0 based).
            pub page: Option<u32>,
            /// Number of elements per page to get back.
            #page_size_attributes
            pub page_size: Option<u32>,
            #sort_parameter

            #(
                #[doc = #field_docs]
                pub #field_idents: Option<#field_value_types>
            ),*
        }

        #conversions

        #mirrorable_items

        #page
    }
}

#[cfg(test)]
mod tests {

    /// Pretty-print the generated items, which also asserts they form a valid
    /// Rust file.
    fn pretty(token_stream: proc_macro2::TokenStream) -> String {
        prettyplease::unparse(&syn::parse2::<syn::File>(token_stream).unwrap())
    }

    /// The built-in integrations named `names`, as if written.
    fn integrations(names: &[&str]) -> Vec<crate::integrations::Active> {
        crate::integrations::BUILT_IN
            .into_iter()
            .filter(|integration| names.contains(&integration.name()))
            .map(|integration| crate::integrations::Active {
                integration,
                flag: quote::format_ident!("{}", integration.name()),
            })
            .collect()
    }

    /// This repository's API stack.
    fn all_integrations() -> Vec<crate::integrations::Active> {
        integrations(&["serde", "utoipa", "validator"])
    }

    // The alternatives to utoipa and validator: JSON schemas, and validation
    // without a serializer.
    #[test]
    fn test_code_gen_for_schemars_and_garde() {
        let model: syn::Path = syn::parse_quote!(domain::models::user::UserModel);

        insta::assert_snapshot!(pretty(super::code_gen(
            sortable_ir(),
            &model,
            &integrations(&["schemars", "garde"]),
            &syn::parse_quote!(::queryweave_api)
        )));
    }

    fn sortable_ir() -> crate::lower::filters::IntermediaryRepresentation {
        let idents = crate::conventions::FiltersNames {
            query: quote::format_ident!("UsersQueryDto"),
            filters: quote::format_ident!("UsersFilterOptionsDto"),
            sort_field: quote::format_ident!("UserSortFieldDto"),
            sort_details: quote::format_ident!("UserSortDetailsDto"),
            input: quote::format_ident!("UserDto"),
            page: quote::format_ident!("UsersPageDto"),
            query_params: quote::format_ident!("UsersQueryParams"),
        };
        // `username` is renamed: `#[equivalent_field(user_name)]`.
        crate::lower::filters::IntermediaryRepresentation {
            field_idents: vec![quote::format_ident!("username_sw")],
            equivalent_field_idents: vec![quote::format_ident!("user_name_sw")],
            field_value_types: vec![syn::parse_quote!(String)],
            field_columns: vec![String::from("username")],
            field_operators: vec![crate::analyze::filters::FilterOperator::StartsWith],
            idents,
            sort: Some(crate::lower::filters::Sort {
                variants: vec![quote::format_ident!("Username")],
                fields: vec![quote::format_ident!("username")],
                equivalent_variants: vec![quote::format_ident!("UserName")],
            }),
        }
    }

    // A plain mirror names neither serde, utoipa nor validator, and its sort
    // direction is the core runtime's.
    #[test]
    fn test_code_gen_for_plain() {
        let model: syn::Path = syn::parse_quote!(domain::models::user::UserModel);

        insta::assert_snapshot!(pretty(super::code_gen(
            sortable_ir(),
            &model,
            &[],
            &syn::parse_quote!(::queryweave)
        )));
    }

    #[test]
    fn test_code_gen_for_sortable() {
        let model: syn::Path = syn::parse_quote!(domain::models::user::UserModel);

        insta::assert_snapshot!(pretty(super::code_gen(
            sortable_ir(),
            &model,
            &all_integrations(),
            &syn::parse_quote!(::queryweave_api)
        )));
    }

    #[test]
    fn test_code_gen_for_unsortable() {
        let idents = crate::conventions::FiltersNames {
            query: quote::format_ident!("UsersQueryDto"),
            filters: quote::format_ident!("UsersFilterOptionsDto"),
            sort_field: quote::format_ident!("UserSortFieldDto"),
            sort_details: quote::format_ident!("UserSortDetailsDto"),
            input: quote::format_ident!("UserDto"),
            page: quote::format_ident!("UsersPageDto"),
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
        let model = syn::parse_str::<syn::Path>("domain::models::user::UserModel").unwrap();

        insta::assert_snapshot!(pretty(super::code_gen(
            intermediary_representation,
            &model,
            &all_integrations(),
            &syn::parse_quote!(::queryweave_api)
        )));
    }
}
