//! The query builders the SQL is written into: one module per builder,
//! implementing the target traits (`crate::target`) for it, each behind the
//! Cargo feature of the same name.

#[cfg(feature = "sea-query")]
mod sea_query;
#[cfg(feature = "sqlx")]
mod sqlx;

#[cfg(feature = "sqlx")]
pub use self::sqlx::BindValues;

/// The column `C` stores a domain field in.
///
/// The backends take the data of a model `M` (a `queryweave::ModelData<M, _>`,
/// whose items only name fields of `M`), and `Columns`' methods check at
/// compile time that every field of `M` has a column on `C`, so the lookup
/// cannot miss. The fallback to the field's own name only satisfies the types
/// (a runtime panic is not allowed here).
#[cfg(any(feature = "sqlx", feature = "sea-query"))]
fn column<C: crate::Columns + ?Sized>(field: &'static str) -> &'static str {
    C::column(field).unwrap_or(field)
}
