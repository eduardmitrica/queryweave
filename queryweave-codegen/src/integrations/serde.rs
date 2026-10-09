//! `serde`: deserializable query parameters, (de)serializable sort types in
//! snake case, and the page alias (`PageDto`, serializable).

use quote::quote;

/// The `serde` integration.
pub struct Serde;

impl super::Integration for Serde {
    fn name(&self) -> &'static str {
        "serde"
    }

    fn names_api_runtime(&self) -> bool {
        true
    }

    /// The check that the API runtime has its `serde` feature.
    fn items(&self, context: &super::Context<'_>) -> proc_macro2::TokenStream {
        super::api_feature_check(context, "Serde")
    }

    fn sort_direction(&self, context: &super::Context<'_>) -> Option<proc_macro2::TokenStream> {
        let runtime = context.runtime;
        Some(quote!(#runtime::SortDirectionDto))
    }

    fn page(&self, context: &super::Context<'_>) -> Option<proc_macro2::TokenStream> {
        let runtime = context.runtime;
        let input = &context.names.input;
        Some(quote!(#runtime::PageDto<#input>))
    }

    fn sort_field(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(serde::Deserialize), quote!(serde::Serialize)],
            attributes: quote!(#[serde(rename_all = "snake_case")]),
        }
    }

    fn sort_details(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(serde::Deserialize), quote!(serde::Serialize)],
            attributes: quote!(),
        }
    }

    fn query_params(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(serde::Deserialize)],
            attributes: quote!(),
        }
    }
}
