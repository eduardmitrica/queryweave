//! The `queryweave` derives under conventions unlike the default ones, the
//! way a crate with its own rules would write its proc-macro crate on top of
//! `queryweave-codegen`. Its tests check that every rule of the conventions
//! reaches the generated code.
//!
//! The rules:
//!
//! - `Filters` names its types after the whole struct name (`Customer` →
//!   `CustomerSearch`, `CustomerSortKey`, ...), so a single-segment name is
//!   fine;
//! - a filter field (and query parameter) puts the operator first
//!   (`sw_name`), except `bool`, which keeps the bare field;
//! - a merge patch struct's name contains `Patch`;
//! - the runtime crates are reached through the using crate's own modules
//!   `crate::runtime` (for `queryweave`) and `crate::api_runtime` (for
//!   `queryweave_api`), as when they are re-exported;
//! - next to the built-in integrations, a mirror may opt into `wire`, a toy
//!   serializer standing for one other than serde: its own sort direction
//!   (`crate::wire::Direction`) and page (`crate::wire::Page`) types, `Ord`
//!   sort fields, and `crate::wire::Encode` impls on the query parameters
//!   and sort details.

use proc_macro::TokenStream;
use queryweave_codegen::conventions::{Conventions, DefaultConventions, FiltersNames};
use queryweave_codegen::integrations::{Context, Integration, TypeAdditions};
use quote::quote;

/// The `wire` integration (see the crate documentation), whose generated code
/// names the using crate's `crate::wire` module.
struct Wire;

impl Integration for Wire {
    fn name(&self) -> &'static str {
        "wire"
    }

    fn sort_direction(&self, _context: &Context<'_>) -> Option<proc_macro2::TokenStream> {
        Some(quote!(crate::wire::Direction))
    }

    fn page(&self, context: &Context<'_>) -> Option<proc_macro2::TokenStream> {
        let input = &context.names.input;
        Some(quote!(crate::wire::Page<#input>))
    }

    fn sort_field(&self, _context: &Context<'_>) -> TypeAdditions {
        TypeAdditions {
            derives: vec![quote!(PartialOrd), quote!(Ord)],
            attributes: quote!(),
        }
    }

    fn items(&self, context: &Context<'_>) -> proc_macro2::TokenStream {
        let query_params = &context.names.query_params;
        quote! {
            impl crate::wire::Encode for #query_params {
                fn encode(&self) -> String {
                    format!("page={:?};page_size={:?}", self.page, self.page_size)
                }
            }
        }
    }

    fn sortable_items(&self, context: &Context<'_>) -> proc_macro2::TokenStream {
        let sort_details = &context.names.sort_details;
        quote! {
            impl crate::wire::Encode for #sort_details {
                fn encode(&self) -> String {
                    format!("{:?}:{}", self.field, crate::wire::Encode::encode(&self.order))
                }
            }
        }
    }
}

/// The conventions of this crate (see the crate documentation).
struct SearchConventions;

impl Conventions for SearchConventions {
    fn filters_names(input: &syn::Ident) -> syn::Result<FiltersNames> {
        Ok(FiltersNames {
            query: quote::format_ident!("{input}Search"),
            filters: quote::format_ident!("{input}Filters"),
            sort_field: quote::format_ident!("{input}SortKey"),
            sort_details: quote::format_ident!("{input}Sort"),
            input: input.clone(),
            page: quote::format_ident!("{input}Page"),
            query_params: quote::format_ident!("{input}SearchParams"),
        })
    }

    fn check_model_name(_name: &syn::Ident) -> syn::Result<()> {
        Ok(())
    }

    fn check_merge_patch_name<S: quote::ToTokens + ?Sized>(
        name: &syn::Ident,
        span: &S,
        what: &str,
    ) -> syn::Result<()> {
        if name.to_string().contains("Patch") {
            Ok(())
        } else {
            queryweave_codegen::errors::abort_spanned(
                span,
                &format!("The {what}'s name `{name}` must contain `Patch`"),
                "Rename it, e.g. `CustomerPatch`",
            )
        }
    }

    fn sort_variant(field: &syn::Ident, span: proc_macro2::Span) -> syn::Ident {
        DefaultConventions::sort_variant(field, span)
    }

    fn filter_field(field: &syn::Ident, operator: &str, span: proc_macro2::Span) -> syn::Ident {
        if operator == "bool" {
            quote::format_ident!("{field}", span = span)
        } else {
            quote::format_ident!("{operator}_{field}", span = span)
        }
    }

    fn check_wrapper_name<S: quote::ToTokens + ?Sized>(
        name: &syn::Ident,
        span: &S,
        what: &str,
    ) -> syn::Result<()> {
        DefaultConventions::check_wrapper_name(name, span, what)
    }

    fn core_crate() -> syn::Path {
        syn::parse_quote!(crate::runtime)
    }

    fn api_crate() -> syn::Path {
        syn::parse_quote!(crate::api_runtime)
    }

    fn integrations() -> Vec<&'static dyn Integration> {
        queryweave_codegen::integrations::BUILT_IN
            .into_iter()
            .chain([&Wire as &'static dyn Integration])
            .collect()
    }
}

/// `Filters` under this crate's conventions (see the crate documentation).
#[proc_macro_derive(
    Filters,
    attributes(queryweave, filter, sortable, filters, equivalent_field)
)]
pub fn derive_filters(input: TokenStream) -> TokenStream {
    queryweave_codegen::filters::<SearchConventions>(input.into()).into()
}

/// `MergePatch` under this crate's conventions (see the crate documentation).
#[proc_macro_derive(MergePatch, attributes(queryweave, merge_patch))]
pub fn derive_merge_patch(input: TokenStream) -> TokenStream {
    queryweave_codegen::merge_patch::<SearchConventions>(input.into()).into()
}

/// `MergePatchNullableFields` under this crate's conventions (see the crate documentation).
#[proc_macro_derive(MergePatchNullableFields, attributes(queryweave, merge_patch))]
pub fn derive_merge_patch_nullable_fields(input: TokenStream) -> TokenStream {
    queryweave_codegen::merge_patch_nullable_fields::<SearchConventions>(input.into()).into()
}

/// `ValidateNonNullable` under this crate's conventions (see the crate documentation).
#[proc_macro_derive(
    ValidateNonNullable,
    attributes(queryweave, validate_non_nullable, equivalent_field)
)]
pub fn derive_validate_non_nullable(input: TokenStream) -> TokenStream {
    queryweave_codegen::validate_non_nullable::<SearchConventions>(input.into()).into()
}
