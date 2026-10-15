//! The `queryweave` derives, under the default conventions.
//!
//! Each derive forwards to its function in `queryweave-codegen` with
//! `DefaultConventions`; see there for what each one generates. A crate
//! wanting other naming rules or runtime paths writes its own proc-macro crate
//! calling those functions with its own conventions.

use proc_macro::TokenStream;
use queryweave_codegen::conventions::DefaultConventions;

/// The filtering, sorting and paging types of a root model, or of a struct
/// mirroring a model (`#[filters(mirrors = path)]`: a root model, or another
/// mirror); see `queryweave_codegen::filters`.
#[proc_macro_derive(
    Filters,
    attributes(queryweave, filter, sortable, filters, equivalent_field)
)]
pub fn derive_filters(input: TokenStream) -> TokenStream {
    queryweave_codegen::filters::<DefaultConventions>(input.into()).into()
}

#[cfg(feature = "merge-patch")]
/// Merge patching a model with its merge patch model
/// (`#[merge_patch(patch = path)]`); see `queryweave_codegen::merge_patch`.
#[proc_macro_derive(MergePatch, attributes(queryweave, merge_patch))]
pub fn derive_merge_patch(input: TokenStream) -> TokenStream {
    queryweave_codegen::merge_patch::<DefaultConventions>(input.into()).into()
}

#[cfg(feature = "merge-patch")]
/// The nullable (`Option<Option<T>>`) fields of a merge patch model; see
/// `queryweave_codegen::merge_patch_nullable_fields`.
#[proc_macro_derive(MergePatchNullableFields, attributes(queryweave, merge_patch))]
pub fn derive_merge_patch_nullable_fields(input: TokenStream) -> TokenStream {
    queryweave_codegen::merge_patch_nullable_fields::<DefaultConventions>(input.into()).into()
}

/// `queryweave::RawValue` for a value object; see
/// `queryweave_codegen::raw_value`.
#[proc_macro_derive(RawValue, attributes(queryweave))]
pub fn derive_raw_value(input: TokenStream) -> TokenStream {
    queryweave_codegen::raw_value::<DefaultConventions>(input.into()).into()
}

#[cfg(feature = "merge-patch")]
/// The non-nullable validation of a merge patch DTO mirroring a domain merge
/// patch model; see `queryweave_codegen::validate_non_nullable`.
///
/// `#[validate_non_nullable(mirrors = path)]`, plus the validation library's
/// flag, e.g. `validator`.
#[proc_macro_derive(
    ValidateNonNullable,
    attributes(queryweave, validate_non_nullable, equivalent_field)
)]
pub fn derive_validate_non_nullable(input: TokenStream) -> TokenStream {
    queryweave_codegen::validate_non_nullable::<DefaultConventions>(input.into()).into()
}

/// The SQL of a wrapper of a domain query model; see
/// `queryweave_codegen::append_to_query`.
#[proc_macro_derive(AppendToQuery, attributes(queryweave, append_to_query))]
pub fn derive_append_to_query(input: TokenStream) -> TokenStream {
    queryweave_codegen::append_to_query::<DefaultConventions>(input.into()).into()
}

/// The SQL of a wrapper of a domain filter-options model; see
/// `queryweave_codegen::append_conditions`.
#[proc_macro_derive(AppendConditions, attributes(queryweave, append_conditions))]
pub fn derive_append_conditions(input: TokenStream) -> TokenStream {
    queryweave_codegen::append_conditions::<DefaultConventions>(input.into()).into()
}

/// `queryweave_sql::Columns` for a `sqlx` entity
/// (`#[columns(model = Model)]`); see `queryweave_codegen::columns`.
#[proc_macro_derive(Columns, attributes(queryweave, columns, domain_field))]
pub fn derive_columns(input: TokenStream) -> TokenStream {
    queryweave_codegen::columns::<DefaultConventions>(input.into()).into()
}
