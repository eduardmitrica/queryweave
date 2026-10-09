//! A page of items, as returned by a paginated query.

// Scoped to this module: a derive's output is a separate item, out of reach of
// an attribute on the struct.
#![cfg_attr(
    feature = "utoipa",
    allow(
        clippy::option_if_let_else,
        reason = "utoipa's `ToSchema` expands the generic `items` field's schema to an if-let/else"
    )
)]

// Unbounded: each integration's derive bounds its own impl (e.g. `Serialize`
// for `T: Serialize`). Not part of the doc comment, which utoipa publishes.
/// A page of items, as returned by a paginated query.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PageDto<T> {
    /// The page returned (0 based), as requested.
    pub page: Option<u32>,
    /// The number of items per page, as requested.
    pub page_size: Option<u32>,
    /// The number of pages (none without a page size).
    pub total_pages: Option<u32>,
    /// The number of items over every page.
    pub total_items: u32,
    /// The items of the page.
    pub items: Vec<T>,
}

impl<T> PageDto<T> {
    /// A page of `items`, out of `total_items`, computing the number of pages.
    #[must_use]
    pub fn new(page: Option<u32>, page_size: Option<u32>, total_items: u32, items: Vec<T>) -> Self {
        // A zero page size has no page count (and must not divide by zero).
        let total_pages = page_size
            .filter(|&page_size| page_size != 0)
            .map(|page_size| total_items.div_ceil(page_size));

        Self {
            page,
            page_size,
            total_pages,
            total_items,
            items,
        }
    }
}
