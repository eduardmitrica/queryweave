//! How a type mirroring a `Filters` model (e.g. an API DTO) converts into the
//! model's generated types without naming them.
//!
//! A mirror names the model it converts into (`#[filters(mirrors = path)]`),
//! never the model's generated types: those are reached through
//! [`Filterable`], so each layer is free to name its own generated types.
//!
//! The mirrored model is a root model or itself a mirror, so mirrors chain
//! (e.g. a gRPC DTO mirroring an API DTO mirroring a domain model): both kinds
//! implement [`Filterable`], and a mirror builds its target's query through
//! [`Filterable::query`] rather than a struct literal, since a root's query
//! and a mirror's (its query parameters) are shaped differently. Every chain
//! ends at a root model ([`Filterable::Root`]); a cycle cannot resolve it, so
//! it fails to compile.
//!
//! A mirror cannot `impl From<Dto> for <Model as Filterable>::Query`: coherence
//! cannot rule out that the projection is `Dto` itself, so the impl overlaps
//! `impl<T> From<T> for T`. Instead the mirror implements the role traits below
//! (whose impl headers name only the model) and the model's derive provides the
//! `From` impls, generic over every type implementing the role for that model,
//! so callers still convert with `.into()`, one hop at a time, or straight to
//! the root's query with [`IntoRootQuery::into_root_query`].

/// The types `#[derive(Filters)]` generates for a root model or a mirror, so
/// code mirroring it names them through it instead of by name.
///
/// Implemented by the derive.
pub trait Filterable {
    /// The root model of the chain: the model itself for a root model, the
    /// mirrored model's root for a mirror.
    type Root: Filterable;
    /// The query: paging, sorting and the filters (a root's query model, a
    /// mirror's query parameters).
    type Query;
    /// The filter options.
    type FilterOptions;
    /// The sortable-fields enum.
    type SortField;
    /// The sort details: a sort field and its direction.
    type SortDetails;

    /// Build a query from its parts.
    fn query(
        page: Option<u32>,
        page_size: Option<u32>,
        sort: Option<Vec<Self::SortDetails>>,
        filters: Self::FilterOptions,
    ) -> Self::Query;

    /// Convert a query along the chain of mirrored models into the root
    /// model's query (the query itself for a root model).
    fn into_root_query(query: Self::Query) -> <Self::Root as Filterable>::Query;
}

/// The query of `M` (a root's query model, or a mirror's query parameters),
/// converting along its chain of mirrored models into the root model's query.
///
/// Implemented by the derive for every `Filters` struct's query; the root
/// model's query converts into itself. `M` is a parameter (rather than found
/// through `Filterable`) so the impl is no more visible than `M`, which may be
/// private while its generated query is public; with a single impl per query,
/// `query.into_root_query()` infers it.
pub trait IntoRootQuery<M: Filterable> {
    /// Convert into the root model's query.
    fn into_root_query(self) -> <M::Root as Filterable>::Query;
}

/// A type converting into model `M`'s query model.
///
/// The model's derive provides `From<T> for <M as Filterable>::Query` for every
/// such `T`.
pub trait QueryOf<M: Filterable> {
    /// Convert into the model's query model.
    fn into_query(self) -> M::Query;
}

/// A type converting into model `M`'s filter options model.
///
/// The model's derive provides `From<T> for <M as Filterable>::FilterOptions`
/// for every such `T`.
pub trait FilterOptionsOf<M: Filterable> {
    /// Convert into the model's filter options model.
    fn into_filter_options(self) -> M::FilterOptions;
}

/// A sortable-fields enum mirroring model `M`'s, variant for variant.
///
/// The model's derive provides `From<T> for <M as Filterable>::SortField` for
/// every such `T`; the way back is [`SortFieldOf::from_sort_field`], since the
/// mirror cannot implement `From` for its own type from a projection either.
pub trait SortFieldOf<M: Filterable>: Sized {
    /// Convert into the model's sort field.
    fn into_sort_field(self) -> M::SortField;

    /// Convert from the model's sort field.
    fn from_sort_field(field: M::SortField) -> Self;
}

/// A type converting into model `M`'s sort details.
///
/// The model's derive provides `From<T> for <M as Filterable>::SortDetails`
/// for every such `T`.
pub trait SortDetailsOf<M: Filterable> {
    /// Convert into the model's sort details.
    fn into_sort_details(self) -> M::SortDetails;
}
