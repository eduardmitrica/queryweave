//! How the derives name what they generate, and which names they require of
//! the structs they are placed on or link to.
//!
//! Every naming rule goes through a [`Conventions`] implementation, chosen
//! by the derive's entry point (`DefaultConventions` for this project's
//! derives), so the rules can be swapped without touching a derive's steps.
//!
//! A derive never rebuilds another type's generated *type* names from these
//! rules: it links to the path the user wrote (see `queryweave::Filterable`),
//! so each layer may name its types under different conventions. Generated
//! *member* names (sort variants, filter fields) are different: a mirror (an
//! API DTO) names the mirrored model's members by applying its own conventions to
//! the model's field names, so the layers must agree on
//! [`Conventions::sort_variant`] and [`Conventions::filter_field`].
//!
//! The conventions also say where the runtime crates the generated code
//! names live ([`Conventions::core_crate`], [`Conventions::api_crate`],
//! [`Conventions::sql_crate`]), so a crate that renames or re-exports them
//! can still use the derives; `#[queryweave(crate = path)]` overrides them on
//! a single struct. Each derive names exactly one runtime crate: the one of its
//! layer (`queryweave-api` and `queryweave-sql` re-export the core).
//!
//! Last, they list the integrations a `Filters` mirror may opt into
//! ([`Conventions::integrations`], see [`crate::integrations`]).

use quote::{ToTokens, format_ident};

/// The names of the types `#[derive(Filters)]` generates for a struct (a
/// domain model or a DTO).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FiltersNames {
    /// The query model: paging, sorting and the filter options.
    pub query: syn::Ident,
    /// The filter options model.
    pub filters: syn::Ident,
    /// The sortable-fields enum.
    pub sort_field: syn::Ident,
    /// The sort details: a sort field and its direction.
    pub sort_details: syn::Ident,
    /// The struct the derive is placed on.
    pub input: syn::Ident,
    /// The page of the struct.
    pub page: syn::Ident,
    /// The query parameters (API `Filters` only).
    pub query_params: syn::Ident,
}

/// The naming rules of the derives.
///
/// Its functions report a name that does not fit through
/// [`crate::errors`], at what the user wrote: the struct's name, or the path
/// inside the attribute naming another struct.
pub trait Conventions {
    /// The names of the types `#[derive(Filters)]` generates for `input`.
    ///
    /// # Errors
    ///
    /// Returns an error, at `input`, when the struct's name does not fit.
    fn filters_names(input: &syn::Ident) -> syn::Result<FiltersNames>;

    /// Check the name of a model deriving `MergePatch`.
    ///
    /// # Errors
    ///
    /// Returns an error, at `name`, when it does not fit.
    fn check_model_name(name: &syn::Ident) -> syn::Result<()>;

    /// Check the name of a merge patch struct: the struct a derive is placed
    /// on (`MergePatchNullableFields`, `ValidateNonNullable`), or one an
    /// attribute links to (`#[merge_patch(patch = path)]`, or
    /// `#[validate_non_nullable(mirrors = path)]`), calling it
    /// `what`.
    ///
    /// # Errors
    ///
    /// Returns an error, at `span`, when the name does not fit.
    fn check_merge_patch_name<S: ToTokens + ?Sized>(
        name: &syn::Ident,
        span: &S,
        what: &str,
    ) -> syn::Result<()>;

    /// The variant of the generated sortable-fields enum for the `#[sortable]`
    /// field `field`, spanned to `span`.
    ///
    /// Shared across layers: a mirror names the mirrored model's variants with
    /// it.
    fn sort_variant(field: &syn::Ident, span: proc_macro2::Span) -> syn::Ident;

    /// The generated filter field (and, on a DTO, query parameter) filtering
    /// `field` with the operator written `operator` in `#[filter(...)]` (`lt`,
    /// `eq`, `gt`, `sw`, `c`, `ew` or `bool`), spanned to `span`.
    ///
    /// Shared across layers: a mirror names the mirrored model's filter fields
    /// with it.
    fn filter_field(field: &syn::Ident, operator: &str, span: proc_macro2::Span) -> syn::Ident;

    /// Check the name of a wrapper an attribute links to
    /// (`#[append_to_query(filters = path)]` of `AppendToQuery`), calling it
    /// `what`.
    ///
    /// # Errors
    ///
    /// Returns an error, at `span`, when the name does not fit.
    fn check_wrapper_name<S: ToTokens + ?Sized>(
        name: &syn::Ident,
        span: &S,
        what: &str,
    ) -> syn::Result<()>;

    /// The path of the core runtime crate, named by the code generated for a
    /// root model (`Filters`, `MergePatch`, `MergePatchNullableFields`,
    /// `RawValue`).
    ///
    /// Defaults to `::queryweave`: absolute, so a local item of the same name
    /// cannot shadow it. Override it when the crate is renamed or re-exported.
    #[must_use]
    fn core_crate() -> syn::Path {
        syn::parse_quote!(::queryweave)
    }

    /// The path of the API runtime crate, named by the code generated for an
    /// API type (a `Filters` mirror with `serde` or `utoipa`,
    /// `ValidateNonNullable`). Defaults to `::queryweave_api`.
    #[must_use]
    fn api_crate() -> syn::Path {
        syn::parse_quote!(::queryweave_api)
    }

    /// The path of the SQL runtime crate, named by the code generated for the
    /// persistence layer (`Columns`, `AppendToQuery`,
    /// `AppendConditions`). Defaults to `::queryweave_sql`.
    #[must_use]
    fn sql_crate() -> syn::Path {
        syn::parse_quote!(::queryweave_sql)
    }

    /// The integrations a `Filters` mirror may opt into, each with the flag
    /// of `#[filters(...)]` named after it, in the order their additions are
    /// generated. Their names must differ from each other and from `mirrors`.
    ///
    /// Defaults to the built-in ones ([`crate::integrations::BUILT_IN`]:
    /// `serde`, `utoipa`, `validator`). Override it to add a crate's own (e.g.
    /// a GraphQL integration) or to replace some (e.g. serde by another
    /// serializer).
    #[must_use]
    fn integrations() -> Vec<&'static dyn crate::integrations::Integration> {
        crate::integrations::BUILT_IN.to_vec()
    }
}

/// This project's conventions.
///
/// `Filters` names its types after the struct's pascal-case segments, the
/// last one being the layer's suffix (`OrderModel` → `OrdersQueryModel`,
/// `OrderSortFieldModel`, ...; `OrderDto` → `OrdersQueryParams`, ...), so the
/// struct's name needs at least two segments, as does a model deriving
/// `MergePatch`. A merge patch struct is named `MergePatch…`
/// (`MergePatchOrderModel`), and a wrapper is pascal cased.
///
/// The runtime crates keep their default, absolute paths (`::queryweave`,
/// ...).
///
/// A sort variant is its field pascal cased (`delivery_date` →
/// `DeliveryDate`), and a filter field is its field followed by the operator
/// (`username_sw`), except `bool`, which keeps the bare field (`is_active`).
pub struct DefaultConventions;

impl Conventions for DefaultConventions {
    fn filters_names(input: &syn::Ident) -> syn::Result<FiltersNames> {
        let parts = crate::names::pascal_case_segments(input, input, "struct", 2)?;
        // At least two segments, so there is a last one.
        let (last, init) = parts
            .split_last()
            .map_or(("", &[][..]), |(last, init)| (last.as_str(), init));
        let without_last = init.join("");

        Ok(FiltersNames {
            query: format_ident!("{without_last}sQuery{last}"),
            filters: format_ident!("{without_last}sFilterOptions{last}"),
            sort_field: format_ident!("{without_last}SortField{last}"),
            sort_details: format_ident!("{without_last}SortDetails{last}"),
            input: input.clone(),
            page: format_ident!("{without_last}sPage{last}"),
            query_params: format_ident!("{without_last}sQueryParams"),
        })
    }

    fn check_model_name(name: &syn::Ident) -> syn::Result<()> {
        crate::names::pascal_case_segments(name, name, "struct", 2).map(drop)
    }

    fn check_merge_patch_name<S: ToTokens + ?Sized>(
        name: &syn::Ident,
        span: &S,
        what: &str,
    ) -> syn::Result<()> {
        crate::names::merge_patch_segments(name, span, what).map(drop)
    }

    fn sort_variant(field: &syn::Ident, span: proc_macro2::Span) -> syn::Ident {
        let pascal_cased = field
            .to_string()
            .split('_')
            .map(crate::utils::uppercase_first_letter)
            .collect::<String>();

        format_ident!("{pascal_cased}", span = span)
    }

    fn filter_field(field: &syn::Ident, operator: &str, span: proc_macro2::Span) -> syn::Ident {
        if operator == "bool" {
            format_ident!("{field}", span = span)
        } else {
            format_ident!("{field}_{operator}", span = span)
        }
    }

    fn check_wrapper_name<S: ToTokens + ?Sized>(
        name: &syn::Ident,
        span: &S,
        what: &str,
    ) -> syn::Result<()> {
        crate::names::pascal_case_segments(name, span, what, 1).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::{Conventions, DefaultConventions, FiltersNames};
    use pretty_assertions::assert_eq;
    use quote::format_ident;

    #[test]
    fn the_default_filters_names_follow_the_struct_segments() {
        assert_eq!(
            DefaultConventions::filters_names(&format_ident!("OrderLineModel")).unwrap(),
            FiltersNames {
                query: format_ident!("OrderLinesQueryModel"),
                filters: format_ident!("OrderLinesFilterOptionsModel"),
                sort_field: format_ident!("OrderLineSortFieldModel"),
                sort_details: format_ident!("OrderLineSortDetailsModel"),
                input: format_ident!("OrderLineModel"),
                page: format_ident!("OrderLinesPageModel"),
                query_params: format_ident!("OrderLinesQueryParams"),
            }
        );
    }

    #[test]
    fn the_default_filters_names_need_two_segments() {
        assert!(DefaultConventions::filters_names(&format_ident!("Order")).is_err());
        assert!(DefaultConventions::filters_names(&format_ident!("order_model")).is_err());
    }

    #[test]
    fn the_default_model_name_needs_two_segments() {
        assert!(DefaultConventions::check_model_name(&format_ident!("OrderModel")).is_ok());
        assert!(DefaultConventions::check_model_name(&format_ident!("Order")).is_err());
    }

    #[test]
    fn the_default_merge_patch_name_starts_with_merge_patch() {
        let name = format_ident!("MergePatchOrderModel");
        assert!(DefaultConventions::check_merge_patch_name(&name, &name, "struct").is_ok());

        let name = format_ident!("OrderPatch");
        assert!(DefaultConventions::check_merge_patch_name(&name, &name, "struct").is_err());
    }

    #[test]
    fn the_default_sort_variant_is_the_field_pascal_cased() {
        let span = proc_macro2::Span::call_site();

        assert_eq!(
            DefaultConventions::sort_variant(&format_ident!("delivery_date"), span),
            format_ident!("DeliveryDate")
        );
        assert_eq!(
            DefaultConventions::sort_variant(&format_ident!("value"), span),
            format_ident!("Value")
        );
    }

    #[test]
    fn the_default_filter_field_is_suffixed_with_the_operator_except_bool() {
        let span = proc_macro2::Span::call_site();
        let field = format_ident!("username");

        assert_eq!(
            DefaultConventions::filter_field(&field, "sw", span),
            format_ident!("username_sw")
        );
        assert_eq!(
            DefaultConventions::filter_field(&field, "bool", span),
            format_ident!("username")
        );
    }

    #[test]
    fn the_default_runtime_crates_are_named_by_absolute_path() {
        let path = |path: syn::Path| quote::ToTokens::to_token_stream(&path).to_string();

        assert_eq!(path(DefaultConventions::core_crate()), ":: queryweave");
        assert_eq!(path(DefaultConventions::api_crate()), ":: queryweave_api");
        assert_eq!(path(DefaultConventions::sql_crate()), ":: queryweave_sql");
    }

    #[test]
    fn the_default_wrapper_name_is_pascal_cased() {
        let name = format_ident!("OrdersQueryModelWrapper");
        assert!(DefaultConventions::check_wrapper_name(&name, &name, "wrapper").is_ok());

        let name = format_ident!("orders_wrapper");
        assert!(DefaultConventions::check_wrapper_name(&name, &name, "wrapper").is_err());
    }
}
