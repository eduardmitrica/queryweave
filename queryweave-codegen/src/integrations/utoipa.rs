//! `utoipa`: schemas for the API documentation (`ToSchema`, `IntoParams`),
//! stating the wire format itself (snake case sort variants, `sort` as a deep
//! object) rather than reading serde's attributes.

use quote::quote;

/// The `utoipa` integration.
pub struct Utoipa;

impl super::Integration for Utoipa {
    fn name(&self) -> &'static str {
        "utoipa"
    }

    fn names_api_runtime(&self) -> bool {
        true
    }

    /// The check that the API runtime has its `utoipa` feature.
    fn items(&self, context: &super::Context<'_>) -> proc_macro2::TokenStream {
        super::api_feature_check(context, "Utoipa")
    }

    fn sort_direction(&self, context: &super::Context<'_>) -> Option<proc_macro2::TokenStream> {
        let runtime = context.runtime;
        Some(quote!(#runtime::SortDirectionDto))
    }

    fn sort_field(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(utoipa::ToSchema)],
            attributes: quote!(#[schema(rename_all = "snake_case")]),
        }
    }

    fn sort_details(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(utoipa::ToSchema)],
            attributes: quote!(),
        }
    }

    fn query_params(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(utoipa::IntoParams), quote!(utoipa::ToSchema)],
            attributes: quote!(#[into_params(parameter_in = Query)]),
        }
    }

    fn sort(&self, _context: &super::Context<'_>) -> proc_macro2::TokenStream {
        quote!(#[param(style = DeepObject, explode)])
    }
}
