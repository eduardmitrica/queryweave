//! Binding the domain's raw values, whatever the database.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use queryweave::Value;
use sqlx::query_builder::Separated;
use sqlx::{Database, Encode, QueryBuilder, Type};

/// A database the SQL helpers can write for.
///
/// It is one whose driver binds every raw value a domain model may filter on
/// or store ([`Value`]'s variants: `bool`, `i16`, `i32`, `i64`, `f32`, `f64`,
/// `String` and the `chrono` date/time types, each possibly `NULL`).
///
/// Implemented for every such sqlx database: Postgres, MySQL/MariaDB and
/// SQLite, with sqlx's `chrono` feature (`sqlx::Any` is not one, having no
/// date/time types). Code generic over the database only needs
/// `DB: BindValues`.
pub trait BindValues: Database {
    /// Bind `value` with its own SQL type; an absent value binds as a `NULL`
    /// of that type.
    fn push_bind_value(query_builder: &mut QueryBuilder<'_, Self>, value: Value);

    /// Bind `value` like [`BindValues::push_bind_value`], into a separated
    /// list without a separator before it (e.g. right after `column = `).
    fn push_bind_value_unseparated<Sep: std::fmt::Display>(
        separated: &mut Separated<'_, '_, Self, Sep>,
        value: Value,
    );
}

/// Bind a [`Value`] with the method `$bind` of `$target` (a `QueryBuilder`'s
/// `push_bind` or a `Separated`'s `push_bind_unseparated`).
macro_rules! bind_value {
    ($target:expr, $bind:ident, $value:expr) => {
        match $value {
            Value::Bool(value) => $target.$bind(value),
            Value::I16(value) => $target.$bind(value),
            Value::I32(value) => $target.$bind(value),
            Value::I64(value) => $target.$bind(value),
            Value::F32(value) => $target.$bind(value),
            Value::F64(value) => $target.$bind(value),
            Value::Text(value) => $target.$bind(value),
            Value::DateTimeUtc(value) => $target.$bind(value),
            Value::NaiveDateTime(value) => $target.$bind(value),
            Value::NaiveDate(value) => $target.$bind(value),
            Value::NaiveTime(value) => $target.$bind(value),
        }
    };
}

// sqlx implements `Encode` for `Option<T>` per database, not generically, so
// the bounds name the `Option`s the values are bound as.
impl<DB> BindValues for DB
where
    DB: Database,
    for<'q> Option<bool>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<i16>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<i32>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<i64>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<f32>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<f64>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<String>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<DateTime<Utc>>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<NaiveDateTime>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<NaiveDate>: Encode<'q, DB> + Type<DB>,
    for<'q> Option<NaiveTime>: Encode<'q, DB> + Type<DB>,
{
    fn push_bind_value(query_builder: &mut QueryBuilder<'_, Self>, value: Value) {
        bind_value!(query_builder, push_bind, value);
    }

    fn push_bind_value_unseparated<Sep: std::fmt::Display>(
        separated: &mut Separated<'_, '_, Self, Sep>,
        value: Value,
    ) {
        bind_value!(separated, push_bind_unseparated, value);
    }
}
