//! `schemars`: JSON schemas (`JsonSchema`) of the query parameters and sort
//! types, stating the wire format itself (snake case sort variants) rather
//! than reading serde's attributes. The schemas `aide`, `okapi` or `dropshot`
//! build their API documentation on.

use quote::quote;

/// The `schemars` integration.
pub struct Schemars;

impl super::Integration for Schemars {
    fn name(&self) -> &'static str {
        "schemars"
    }

    fn names_api_runtime(&self) -> bool {
        true
    }

    /// The check that the API runtime has its `schemars` feature.
    fn items(&self, context: &super::Context<'_>) -> proc_macro2::TokenStream {
        super::api_feature_check(context, "Schemars")
    }

    /// The API's direction, which has a schema (with `queryweave-api`'s
    /// `schemars` feature).
    fn sort_direction(&self, context: &super::Context<'_>) -> Option<proc_macro2::TokenStream> {
        let runtime = context.runtime;
        Some(quote!(#runtime::SortDirectionDto))
    }

    fn sort_field(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(schemars::JsonSchema)],
            attributes: quote!(#[schemars(rename_all = "snake_case")]),
        }
    }

    fn sort_details(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(schemars::JsonSchema)],
            attributes: quote!(),
        }
    }

    fn query_params(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(schemars::JsonSchema)],
            attributes: quote!(),
        }
    }
}
