//! What a `Filters` mirror's generated code integrates with (serialization,
//! API documentation, validation, ...), each opted into per struct with a
//! flag of `#[filters(...)]` named after it.
//!
//! An integration only adds to the mirror's generated types: derives and
//! attributes, field attributes, extra items, and the sort direction and page
//! types it needs. The mirror's code generation names no integration, and no
//! integration relies on another's attributes: each states the wire format
//! itself (e.g. snake case sort variants).
//!
//! The integrations a mirror may opt into are the conventions'
//! ([`crate::conventions::Conventions::integrations`]): the built-in ones
//! ([`BUILT_IN`]: `serde`, `utoipa`, `validator`, `schemars`, `garde`) by
//! default, to which a
//! proc-macro crate adds its own, or which it replaces (e.g. by another
//! serializer).
//!
//! # Examples
//!
//! An integration deriving a GraphQL input type, with its own sort direction
//! (named by path from the using crate, e.g. a type deriving the GraphQL
//! enum and converting both ways with `queryweave::SortDirectionModel`):
//!
//! ```rust
//! use queryweave_codegen::integrations::{Context, Integration, TypeAdditions};
//! use quote::quote;
//!
//! struct Graphql;
//!
//! impl Integration for Graphql {
//!     fn name(&self) -> &'static str {
//!         "graphql"
//!     }
//!
//!     fn sort_direction(&self, _context: &Context<'_>) -> Option<proc_macro2::TokenStream> {
//!         Some(quote!(crate::graphql::SortDirection))
//!     }
//!
//!     fn sort_field(&self, _context: &Context<'_>) -> TypeAdditions {
//!         TypeAdditions {
//!             derives: vec![quote!(async_graphql::Enum)],
//!             attributes: quote!(#[graphql(rename_items = "snake_case")]),
//!         }
//!     }
//! }
//!
//! // Registered next to the built-in ones, in a crate's conventions:
//! let integrations: Vec<&'static dyn Integration> = queryweave_codegen::integrations::BUILT_IN
//!     .into_iter()
//!     .chain([&Graphql as &'static dyn Integration])
//!     .collect();
//! assert_eq!(integrations.len(), queryweave_codegen::integrations::BUILT_IN.len() + 1);
//! ```

mod garde;
mod schemars;
mod serde;
mod utoipa;
mod validator;

/// What an integration knows of the mirror it adds to.
pub struct Context<'a> {
    /// The names of the mirror's generated types.
    pub names: &'a crate::conventions::FiltersNames,
    /// The runtime crate the generated code names.
    pub runtime: &'a syn::Path,
    /// The integration's flag, as written in `#[filters(...)]`: where to
    /// report what concerns the integration (e.g. a disabled feature).
    pub flag: &'a syn::Ident,
}

/// What an integration knows of the merge patch DTO deriving
/// `ValidateNonNullable` it adds to.
pub struct NonNullableContext<'a> {
    /// The DTO.
    pub ident: &'a syn::Ident,
    /// The DTO's fields, by the names clients send.
    pub fields: &'a [syn::Ident],
    /// The runtime crate the generated code names.
    pub runtime: &'a syn::Path,
    /// The integration's flag, as written in `#[validate_non_nullable(...)]`.
    pub flag: &'a syn::Ident,
}

/// What an integration adds to one generated type.
#[derive(Default)]
pub struct TypeAdditions {
    /// The derives, each a path (e.g. `serde::Deserialize`).
    pub derives: Vec<proc_macro2::TokenStream>,
    /// The attributes, written after the derives.
    pub attributes: proc_macro2::TokenStream,
}

/// An integration of a mirror's generated code with another crate.
///
/// Every addition defaults to nothing, so an integration implements only what
/// it adds. Its additions name the crates they need by absolute path or by a
/// path from the using crate, never assuming the call site's imports.
pub trait Integration: Sync {
    /// Its flag in `#[filters(...)]`, e.g. `serde`.
    fn name(&self) -> &'static str;

    /// The integrations it cannot do without, each with the reason, reported
    /// at its flag when one is missing.
    fn requires(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// Whether its additions name the API runtime crate (e.g.
    /// `SortDirectionDto`), which a mirror then names by default.
    fn names_api_runtime(&self) -> bool {
        false
    }

    /// The sort direction type it needs in the sort details, if not the core
    /// `SortDirectionModel`. It converts both ways with `SortDirectionModel`,
    /// for the mirror to convert into the struct it mirrors and back. The
    /// integrations of a mirror providing one must agree on it.
    fn sort_direction(&self, _context: &Context<'_>) -> Option<proc_macro2::TokenStream> {
        None
    }

    /// The type of a page of the mirror's input, for the page alias. The
    /// integrations of a mirror providing one must agree on it.
    fn page(&self, _context: &Context<'_>) -> Option<proc_macro2::TokenStream> {
        None
    }

    /// Its additions to the sort field enum.
    fn sort_field(&self, _context: &Context<'_>) -> TypeAdditions {
        TypeAdditions::default()
    }

    /// Its additions to the sort details struct.
    fn sort_details(&self, _context: &Context<'_>) -> TypeAdditions {
        TypeAdditions::default()
    }

    /// Its additions to the query parameters struct.
    fn query_params(&self, _context: &Context<'_>) -> TypeAdditions {
        TypeAdditions::default()
    }

    /// Its attributes on the `page_size` query parameter.
    fn page_size(&self, _context: &Context<'_>) -> proc_macro2::TokenStream {
        proc_macro2::TokenStream::new()
    }

    /// Its attributes on the `sort` query parameter (of a sortable mirror).
    fn sort(&self, _context: &Context<'_>) -> proc_macro2::TokenStream {
        proc_macro2::TokenStream::new()
    }

    /// Its items of any mirror (e.g. trait impls on the generated types).
    fn items(&self, _context: &Context<'_>) -> proc_macro2::TokenStream {
        proc_macro2::TokenStream::new()
    }

    /// Its items of a sortable mirror (e.g. a validator of the sort details).
    fn sortable_items(&self, _context: &Context<'_>) -> proc_macro2::TokenStream {
        proc_macro2::TokenStream::new()
    }

    /// Whether it applies to `ValidateNonNullable` (a validation library),
    /// defining the DTO's `validate_non_nullable` with
    /// [`Integration::validate_non_nullable`].
    fn supports_validate_non_nullable(&self) -> bool {
        false
    }

    /// The DTO's `validate_non_nullable`: its library's validation of the
    /// struct, then an error for each field of `rejected_nulls()` (the fields
    /// set to `null` that may not be), in its library's error type.
    fn validate_non_nullable(&self, _context: &NonNullableContext<'_>) -> proc_macro2::TokenStream {
        proc_macro2::TokenStream::new()
    }
}

/// The built-in integrations, the default of
/// [`crate::conventions::Conventions::integrations`].
///
/// In the order their additions are generated: `serde` (deserializable query
/// parameters, (de)serializable sort types, the page alias), `utoipa` (API
/// documentation schemas), `validator` (validation, needing `serde`),
/// `schemars` (JSON schemas) and `garde` (validation needing no serializer).
pub const BUILT_IN: [&dyn Integration; 5] = [
    &serde::Serde,
    &utoipa::Utoipa,
    &validator::Validator,
    &schemars::Schemars,
    &garde::Garde,
];

/// An integration a mirror opts into, with the flag written for it.
#[derive(Clone)]
pub(crate) struct Active {
    /// The integration.
    pub integration: &'static dyn Integration,
    /// Its flag, as written (so an error about it can point at it).
    pub flag: syn::Ident,
}

impl std::fmt::Debug for Active {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_tuple("Active").field(&self.flag).finish()
    }
}

impl Active {
    /// What the integration knows of the mirror whose generated types are
    /// named `names`, naming `runtime`.
    pub(crate) const fn context<'a>(
        &'a self,
        names: &'a crate::conventions::FiltersNames,
        runtime: &'a syn::Path,
    ) -> Context<'a> {
        Context {
            names,
            runtime,
            flag: &self.flag,
        }
    }
}

/// A check that the API runtime is built with its feature `feature` (e.g.
/// `Serde`), which the integration naming its types needs: reported at the
/// integration's flag, naming the feature to enable (see the runtime's
/// `features` module), rather than only as traits missing on its types.
fn api_feature_check(context: &Context<'_>, feature: &str) -> proc_macro2::TokenStream {
    let span = context.flag.span();
    let runtime = crate::utils::respanned(context.runtime, span);
    let feature = syn::Ident::new(feature, span);

    quote::quote_spanned! {span=>
        const _: () = {
            const fn requires_feature<T: #runtime::features::#feature>() {}
            requires_feature::<#runtime::SortDirectionDto>();
        };
    }
}

/// Integrations are told apart by name.
impl PartialEq for Active {
    fn eq(&self, other: &Self) -> bool {
        self.integration.name() == other.integration.name() && self.flag == other.flag
    }
}

impl Eq for Active {}

/// Check that `registry`'s integrations can each be told apart by its flag:
/// distinct names, none of them another option of `#[filters(...)]`.
///
/// # Errors
///
/// Returns an error at the derive (the conventions are not the user's code)
/// naming the clashing integration.
pub(crate) fn check_registry(registry: &[&'static dyn Integration]) -> syn::Result<()> {
    for (index, integration) in registry.iter().enumerate() {
        let name = integration.name();
        let clashes = name == "mirrors"
            || registry
                .iter()
                .take(index)
                .any(|earlier| earlier.name() == name);
        if clashes {
            return crate::errors::abort_call_site(
                &format!("The conventions register a second `{name}` option of #[filters(...)]"),
                "Give each integration of `Conventions::integrations` its own name, other than `mirrors`",
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    /// An integration named `name`, adding nothing.
    struct Named(&'static str);

    impl super::Integration for Named {
        fn name(&self) -> &'static str {
            self.0
        }
    }

    static OTHER_SERDE: Named = Named("serde");
    static MIRRORS: Named = Named("mirrors");
    static GRAPHQL: Named = Named("graphql");

    // The built-in integrations, and others added to them, are told apart.
    #[test]
    fn test_check_registry_accepts_distinct_names() {
        assert!(super::check_registry(&super::BUILT_IN).is_ok());
        assert!(super::check_registry(&[super::BUILT_IN[0], &GRAPHQL]).is_ok());
    }

    // A flag must name one integration, and not be another option.
    #[test]
    fn test_check_registry_rejects_clashing_names() {
        assert!(super::check_registry(&[super::BUILT_IN[0], &OTHER_SERDE]).is_err());
        assert!(super::check_registry(&[&MIRRORS]).is_err());
    }

    // What an integration requires is another built-in one.
    #[test]
    fn test_requirements_name_built_in_integrations() {
        for integration in super::BUILT_IN {
            for (required, _) in integration.requires() {
                assert!(
                    super::BUILT_IN
                        .iter()
                        .any(|other| other.name() == *required),
                    "{required}"
                );
            }
        }
    }
}
