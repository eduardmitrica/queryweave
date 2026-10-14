//! The query builders the SQL is written into: one module per builder,
//! implementing the target traits (`crate::target`) for it, each behind the
//! Cargo feature of the same name.

#[cfg(feature = "sqlx")]
mod sqlx;

#[cfg(feature = "sqlx")]
pub use self::sqlx::BindValues;
