//! The code generation behind the `queryweave` derives, as an ordinary
//! library.
//!
//! Each derive is a function from the derive's input to the generated code,
//! generic over the [`conventions::Conventions`] it names things by.
//! `queryweave-derive` exports every derive with
//! [`conventions::DefaultConventions`]; a crate wanting other rules writes its
//! own small proc-macro crate calling these functions with its own
//! conventions:
//!
//! ```rust,ignore
//! #[proc_macro_derive(Filters, attributes(filter, sortable, filters, equivalent_field))]
//! pub fn derive_filters(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//!     queryweave_codegen::filters::<MyConventions>(input.into()).into()
//! }
//! ```
//!
//! Every derive is built as a `steps::Derive` pipeline, private to the crate
//! (see `docs/macros.md`).

// The public API: what a proc-macro crate with its own conventions needs,
// besides the derive functions below.

pub mod conventions;

/// Errors spanned to the user's code, for a `Conventions` impl to report a
/// name that does not fit.
pub mod errors;

// Documented by its own `//!` comment, whose links resolve inside the module.
pub mod integrations;

/// Checks on the names of the structs a derive is placed on or links to.
///
/// The building blocks of `DefaultConventions`, for a `Conventions` impl to
/// reuse.
pub mod names;

// The plumbing of the derives' pipelines, private so it can change freely.

/// Struct and field attributes shared by several derives (e.g.
/// `#[equivalent_field(name)]`).
mod attributes;

/// Compile-time assertions (e.g. that a mapped field exists) for generated code.
mod checks;

/// Mod containing utils.
mod utils;

// Documented by its own `//!` comment, whose links resolve inside the module.
mod steps;

mod analyze;
mod code_gen;
mod lower;
mod mapping;
mod parse;
mod parse_top_level_meta;
mod pipeline;

use conventions::Conventions;
use proc_macro2::TokenStream;

/// `#[derive(Filters)]`: the filtering, sorting and paging types of a struct,
/// from its `#[filter(lt|eq|gt|sw|c|ew|bool)]` and `#[sortable]` fields.
///
/// The struct is either:
///
/// - a **root** model (no `#[filters(mirrors = ...)]`): it gets the query,
///   filter-options, sort-field and sort-details models handing filters and
///   sorting to the persistence layer as data, and publishes them through
///   `queryweave::Filterable`;
/// - a **mirror** of a model (`#[filters(mirrors = path)]`, e.g. an API DTO):
///   it gets its own sort types and query parameters, converting into the
///   mirrored model's types. A plain mirror names only the core runtime; the
///   flags `#[filters(mirrors = path, serde, utoipa, validator)]` opt into
///   deserializable query parameters (and a `PageDto` alias), API
///   documentation schemas and validation (which needs `serde`), and
///   `schemars` and `garde` into their alternatives (JSON schemas, validation
///   needing no serializer), each needing only its own crate. A field
///   named differently on the mirrored model maps onto it with
///   `#[equivalent_field(name)]`; the query parameters keep the mirror's names.
///   The mirrored model is a root model or another mirror (e.g. a gRPC DTO
///   mirroring an API DTO): a mirror publishes its own types through
///   `queryweave::Filterable` too, every chain ending at a root model, and
///   `queryweave::IntoRootQuery` converts a query straight into the root's.
///
/// Attributes: `filter`, `sortable`, `filters`, `equivalent_field`.
#[must_use]
pub fn filters<C: Conventions>(input: TokenStream) -> TokenStream {
    steps::expand::<pipeline::filters::Filters<C>>(input)
}

/// `#[derive(MergePatch)]`: the functions merge patching a model with its
/// merge patch model counterpart.
///
/// Every field not marked `#[merge_patch(skip)]` must exist on the merge patch
/// model as an `Option<P>` where `P: Into<FieldType>`; `Some` replaces the
/// field, `None` leaves it untouched.
///
/// Attributes: `merge_patch` (`patch = path` on the struct, `skip` on a
/// field).
#[must_use]
#[cfg(feature = "merge-patch")]
pub fn merge_patch<C: Conventions>(input: TokenStream) -> TokenStream {
    steps::expand::<pipeline::merge_patch::MergePatch<C>>(input)
}

/// `#[derive(MergePatchNullableFields)]`: a function listing the nullable
/// fields of a struct used for merge-patching, i.e. its `Option<Option<T>>`
/// fields.
#[must_use]
#[cfg(feature = "merge-patch")]
pub fn merge_patch_nullable_fields<C: Conventions>(input: TokenStream) -> TokenStream {
    steps::expand::<pipeline::merge_patch_nullable_fields::MergePatchNullableFields<C>>(input)
}

/// `#[derive(RawValue)]`: `queryweave::RawValue` for a value object (a
/// newtype such as `struct Username<T = String>(T);`), whose raw value is the
/// wrapped type's.
#[must_use]
pub fn raw_value<C: Conventions>(input: TokenStream) -> TokenStream {
    steps::expand::<pipeline::raw_value::RawValue<C>>(input)
}

/// `#[derive(ValidateNonNullable)]`: a validation function for a merge patch
/// DTO, mirroring a domain merge patch model.
///
/// A field named differently on the domain merge patch model maps onto it with
/// `#[equivalent_field(name)]`; validation errors keep the DTO's field names.
/// The mirrored struct is a merge patch model or another mirror (e.g. a gRPC
/// message mirroring the DTO): every one implements `queryweave::NullableFields`
/// under its own names, every chain ending at a merge patch model.
///
/// Attributes: `validate_non_nullable`, `equivalent_field`.
#[must_use]
#[cfg(feature = "merge-patch")]
pub fn validate_non_nullable<C: Conventions>(input: TokenStream) -> TokenStream {
    steps::expand::<pipeline::validate_non_nullable::ValidateNonNullable<C>>(input)
}

/// `#[derive(AppendToQuery)]`: the append to query trait on a wrapper of a
/// domain query model.
///
/// `#[append_to_query(columns = Entity, filters = Wrapper)]` names the entity
/// (deriving `Columns`) whose columns store the query's domain fields, and
/// the wrapper building its `WHERE` clause.
///
/// Attributes: `append_to_query` (`columns`, `filters`).
#[must_use]
pub fn append_to_query<C: Conventions>(input: TokenStream) -> TokenStream {
    steps::expand::<pipeline::append_to_query::AppendToQuery<C>>(input)
}

/// `#[derive(AppendConditions)]`: `queryweave_sql::AppendConditions` for a
/// filter-options wrapper, appending its set filters to a query as conditions.
///
/// `#[append_conditions(columns = Entity)]` names the entity (deriving `Columns`) whose columns
/// store the filtered domain fields.
///
/// Attributes: `append_conditions` (`columns`).
#[must_use]
pub fn append_conditions<C: Conventions>(input: TokenStream) -> TokenStream {
    steps::expand::<pipeline::append_conditions::AppendConditions<C>>(input)
}

/// `#[derive(Columns)]`: `queryweave_sql::Columns` for a `sqlx` entity,
/// recording which column stores each field of the domain model named by
/// `#[columns(model = Model)]`.
///
/// Each entity field is a column (named after the field, or its
/// `#[sqlx(rename = "...")]`) storing the domain field of the same name, or
/// the one named by `#[domain_field(name)]`, which must exist on the domain
/// model. `#[sqlx(skip)]` and `#[sqlx(flatten)]` fields are not columns.
///
/// Attributes: `columns` (`model = path`), `domain_field`.
#[must_use]
pub fn columns<C: Conventions>(input: TokenStream) -> TokenStream {
    steps::expand::<pipeline::columns::Columns<C>>(input)
}
