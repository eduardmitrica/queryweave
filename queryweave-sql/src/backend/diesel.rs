//! Diesel's typed query DSL as the target of a model's data.
//!
//! Filter conditions become boxed expressions on a table, sort keys boxed
//! orderings, a page `LIMIT` / `OFFSET` values, and merge-patch writes typed
//! `column = value` assignments for a changeset.
//!
//! Diesel types every column statically, so a column is named by the code
//! `Columns` generates for a Diesel entity (one `match` arm per column), not
//! looked up by name here. The functions below take that typed column and an
//! item of the model's data, and convert the item's `Value` into the column's
//! own Rust type, picked by the column's SQL type through [`ValueSqlType`]: no
//! SQL is written by hand, and Diesel renders it for each backend.
//!
//! The wrappers reach a Diesel entity's methods through [`Entity`], which
//! `Columns` implements for it, and [`apply`] them to a [`BoxedQuery`].
//!
//! A domain field whose value does not fit its column's SQL type (a domain
//! field and its column disagree on the type, or a nullable field is written
//! to a `NOT NULL` column) fails to build where the entity's methods are
//! called for its model: the domain derives publish each field's value type
//! (`PersistedFields::FIELD_TYPES`), and each column's SQL type the values it
//! takes ([`ValueSqlType::ACCEPTS`], through [`FilterColumn::ACCEPTS`] and
//! [`ColumnValue::TYPE`]). A value that
//! still does not fit at run time (a model implementing `PersistedFields` by
//! hand, or a `NULL` written to a `NOT NULL` column) is a [`ValueMismatch`]
//! error, not a panic.

pub(crate) mod check;

use crate::Junction;
use crate::sql::{LIKE_ESCAPE, like_pattern, page_offset};
use diesel::backend::Backend;
#[cfg(feature = "merge-patch")]
use diesel::expression::TypedExpressionType;
use diesel::expression::expression_types::NotSelectable;
use diesel::expression::{AsExpression, BoxableExpression, Expression};
use diesel::expression_methods::EscapeExpressionMethods;
use diesel::query_dsl::methods::{FilterDsl, LimitDsl, OffsetDsl, ThenOrderDsl};
use diesel::sql_types::{self, Bool, IntoNullable, Nullable, SingleValue, SqlType, is_nullable};
use diesel::{BoolExpressionMethods, ExpressionMethods, NullableExpressionMethods};
use diesel::{TextExpressionMethods, dsl};
#[cfg(feature = "merge-patch")]
use queryweave::FieldValue;
use queryweave::{
    Conditions, FilterCondition, FilterOperator, PersistedFields, SortDirectionModel, SortKey,
    SortKeys, Value, ValueKind,
};

/// A filter condition on the table `QS`, boxed for a boxed query's `filter`.
///
/// Nullable: a comparison with a nullable column, or with a `NULL` value, may
/// be `NULL`.
pub type BoxedCondition<'a, QS, DB> =
    Box<dyn BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a>;

/// An ordering on the table `QS`, boxed for a boxed query's `then_order_by`.
pub type BoxedOrdering<'a, QS, DB> =
    Box<dyn BoxableExpression<QS, DB, SqlType = NotSelectable> + 'a>;

/// A value that does not fit its column: the domain field's raw type and the
/// column's SQL type disagree, or the value is a `NULL` for a `NOT NULL`
/// column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueMismatch {
    /// The domain field whose value does not fit.
    pub field: &'static str,
}

impl std::fmt::Display for ValueMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "the value of the domain field `{}` does not fit the SQL type of its column",
            self.field
        )
    }
}

impl std::error::Error for ValueMismatch {}

/// A non-null SQL type a domain value binds as: the Rust type it binds, taken
/// from a [`Value`].
///
/// Implemented for Diesel's SQL types queryweave's values fit, each accepting
/// the variants it holds without loss (an `Integer` column takes `I32` and
/// `I16`; a `Timestamp` column a `NaiveDateTime`, or a UTC `DateTimeUtc` as its
/// naive UTC time). Implement it for a SQL type of your own (with
/// [`Comparison`] and [`ColumnValue`]) to filter and write its columns.
pub trait ValueSqlType: SqlType<IsNull = is_nullable::NotNull> + SingleValue + Sized {
    /// The [`Value`] variants [`rust`](Self::rust) takes, for the compile-time
    /// check that a domain field's value fits its column.
    const ACCEPTS: &'static [ValueKind];

    /// The Rust type the value binds as.
    type Rust: AsExpression<Self> + Send + 'static;

    /// The value as this SQL type's Rust type (`None` for a `NULL`).
    ///
    /// # Errors
    ///
    /// The value itself, if it is of another type.
    fn rust(value: Value) -> Result<Option<Self::Rust>, Value>;
}

/// Implement [`ValueSqlType`] (and [`Comparison`], [`ColumnValue`]) for SQL
/// types, each listing the `Value` variants it accepts and how each converts.
macro_rules! value_sql_types {
    ($($sql:ty => $rust:ty { $($variant:ident($value:ident) => $convert:expr),+ $(,)? }),+ $(,)?) => {$(
        impl ValueSqlType for $sql {
            const ACCEPTS: &'static [ValueKind] = &[$(ValueKind::$variant),+];

            type Rust = $rust;

            fn rust(value: Value) -> Result<Option<Self::Rust>, Value> {
                match value {
                    $(Value::$variant(value) => Ok(value.map(|$value| $convert)),)+
                    other => Err(other),
                }
            }
        }

        impl<'a, C, QS, DB> Comparison<'a, C, QS, DB> for $sql
        where
            DB: Backend,
            C: Expression + NullableExpressionMethods,
            C::SqlType: IntoNullable<Nullable = Nullable<$sql>>,
            dsl::Nullable<C>: Expression<SqlType = Nullable<$sql>>,
            dsl::Eq<dsl::Nullable<C>, Option<$rust>>:
                BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
            dsl::Lt<dsl::Nullable<C>, Option<$rust>>:
                BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
            dsl::Gt<dsl::Nullable<C>, Option<$rust>>:
                BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
        {
            fn compare(
                column: C,
                operator: FilterOperator,
                value: Value,
            ) -> Option<BoxedCondition<'a, QS, DB>> {
                compare_values::<C, $sql, QS, DB>(column, operator, value)
            }
        }

        impl ColumnValue for $sql {
            const TYPE: ColumnType = ColumnType::not_null::<$sql>();

            type Rust = $rust;

            fn from_value(value: Value) -> Option<Self::Rust> {
                // A `NOT NULL` column cannot be set to `NULL`.
                <$sql as ValueSqlType>::rust(value).ok().flatten()
            }
        }
    )+};
}

value_sql_types!(
    sql_types::Bool => bool { Bool(value) => value },
    sql_types::SmallInt => i16 { I16(value) => value },
    sql_types::Integer => i32 { I32(value) => value, I16(value) => i32::from(value) },
    sql_types::BigInt => i64 {
        I64(value) => value,
        I32(value) => i64::from(value),
        I16(value) => i64::from(value),
    },
    sql_types::Float => f32 { F32(value) => value },
    sql_types::Double => f64 { F64(value) => value, F32(value) => f64::from(value) },
    sql_types::Timestamp => chrono::NaiveDateTime {
        NaiveDateTime(value) => value,
        DateTimeUtc(value) => value.naive_utc(),
    },
    sql_types::Date => chrono::NaiveDate { NaiveDate(value) => value },
    sql_types::Time => chrono::NaiveTime { NaiveTime(value) => value },
);

#[cfg(feature = "diesel-postgres")]
value_sql_types!(
    sql_types::Timestamptz => chrono::DateTime<chrono::Utc> { DateTimeUtc(value) => value },
);

#[cfg(feature = "diesel-sqlite")]
value_sql_types!(
    sql_types::TimestamptzSqlite => chrono::DateTime<chrono::Utc> {
        DateTimeUtc(value) => value,
    },
);

#[cfg(feature = "diesel-mysql")]
value_sql_types!(
    sql_types::Datetime => chrono::NaiveDateTime {
        NaiveDateTime(value) => value,
        DateTimeUtc(value) => value.naive_utc(),
    },
);

// Text is a `ValueSqlType` like the others, but compares with `LIKE` too.
impl ValueSqlType for sql_types::Text {
    const ACCEPTS: &'static [ValueKind] = &[ValueKind::Text];

    type Rust = String;

    fn rust(value: Value) -> Result<Option<Self::Rust>, Value> {
        match value {
            Value::Text(value) => Ok(value),
            other => Err(other),
        }
    }
}

impl ColumnValue for sql_types::Text {
    const TYPE: ColumnType = ColumnType::not_null::<Self>();

    type Rust = String;

    fn from_value(value: Value) -> Option<Self::Rust> {
        <Self as ValueSqlType>::rust(value).ok().flatten()
    }
}

/// How a column whose non-null SQL type is `Self` is compared with a value:
/// `<`, `=`, `>`, and for text the `LIKE` operators.
///
/// Implemented per SQL type, so the code generated for a column is the same
/// call whatever its type: [`condition`] picks the implementation by the
/// column's SQL type.
#[diagnostic::on_unimplemented(
    message = "queryweave cannot compare a domain value with a column of SQL type `{Self}`",
    label = "no `queryweave_sql::diesel::Comparison` for this SQL type",
    note = "mark the entity field of a column storing no domain field `#[columns(skip)]`, or implement `ValueSqlType`, `Comparison` and `ColumnValue` for the SQL type"
)]
pub trait Comparison<'a, C, QS, DB>: ValueSqlType {
    /// `column operator value`, the value bound with the column's type; `None`
    /// if the value is of another type, or the operator does not apply.
    fn compare(
        column: C,
        operator: FilterOperator,
        value: Value,
    ) -> Option<BoxedCondition<'a, QS, DB>>;
}

impl<'a, C, QS, DB> Comparison<'a, C, QS, DB> for sql_types::Text
where
    DB: Backend,
    C: Expression + NullableExpressionMethods,
    C::SqlType: IntoNullable<Nullable = Nullable<Self>>,
    dsl::Nullable<C>: Expression<SqlType = Nullable<Self>> + TextExpressionMethods,
    dsl::Escape<dsl::Like<dsl::Nullable<C>, String>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
    dsl::Eq<dsl::Nullable<C>, Option<String>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
    dsl::Lt<dsl::Nullable<C>, Option<String>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
    dsl::Gt<dsl::Nullable<C>, Option<String>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
{
    fn compare(
        column: C,
        operator: FilterOperator,
        value: Value,
    ) -> Option<BoxedCondition<'a, QS, DB>> {
        // The `Text` filter kind guarantees the `LIKE` operators only ever
        // carry text: an escaped `%` pattern, matched literally.
        if let Value::Text(Some(text)) = &value
            && let Some(pattern) = like_pattern(operator, text)
        {
            return Some(Box::new(
                column.nullable().like(pattern).escape(LIKE_ESCAPE),
            ));
        }

        compare_values::<C, Self, QS, DB>(column, operator, value)
    }
}

/// `column < value`, `=` or `>`, the column made nullable so a `NULL` value
/// (or column) compares as in SQL; `None` for a value of another type, or a
/// `LIKE` operator (text only).
fn compare_values<'a, C, ST, QS, DB>(
    column: C,
    operator: FilterOperator,
    value: Value,
) -> Option<BoxedCondition<'a, QS, DB>>
where
    DB: Backend,
    ST: ValueSqlType,
    Option<ST::Rust>: AsExpression<Nullable<ST>>,
    C: Expression + NullableExpressionMethods,
    C::SqlType: IntoNullable<Nullable = Nullable<ST>>,
    dsl::Nullable<C>: Expression<SqlType = Nullable<ST>>,
    dsl::Eq<dsl::Nullable<C>, Option<ST::Rust>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
    dsl::Lt<dsl::Nullable<C>, Option<ST::Rust>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
    dsl::Gt<dsl::Nullable<C>, Option<ST::Rust>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>> + 'a,
{
    let value = ST::rust(value).ok()?;
    let column = column.nullable();

    match operator {
        FilterOperator::LessThan => Some(Box::new(column.lt(value))),
        FilterOperator::Equal => Some(Box::new(column.eq(value))),
        FilterOperator::GreaterThan => Some(Box::new(column.gt(value))),
        FilterOperator::StartsWith | FilterOperator::Contains | FilterOperator::EndsWith => None,
    }
}

/// `Nullable<ST>` → `ST`: the non-null SQL type of a column, nullable or not
/// (through `IntoNullable`, which leaves a nullable type as it is).
pub trait NonNull {
    /// The non-null SQL type.
    type SqlType;
}

impl<ST: SqlType<IsNull = is_nullable::NotNull>> NonNull for Nullable<ST> {
    type SqlType = ST;
}

/// The non-null SQL type of the column `C`.
pub type NonNullSqlType<C> =
    <<<C as Expression>::SqlType as IntoNullable>::Nullable as NonNull>::SqlType;

/// The condition `column operator value` for a filter condition of the model,
/// the column being the one the entity stores its domain field in.
///
/// # Errors
///
/// [`ValueMismatch`] if the condition's value does not fit the column's SQL
/// type.
pub fn condition<'a, C, QS, DB>(
    column: C,
    filter_condition: FilterCondition,
) -> Result<BoxedCondition<'a, QS, DB>, ValueMismatch>
where
    C: Expression,
    C::SqlType: IntoNullable,
    <C::SqlType as IntoNullable>::Nullable: NonNull,
    NonNullSqlType<C>: Comparison<'a, C, QS, DB>,
{
    let FilterCondition {
        field,
        operator,
        value,
    } = filter_condition;

    <NonNullSqlType<C> as Comparison<'a, C, QS, DB>>::compare(column, operator, value)
        .ok_or(ValueMismatch { field })
}

/// The conditions joined by `junction` (`AND` or `OR`), in order; `None` when
/// there are none.
pub fn join<'a, QS, DB>(
    conditions: impl IntoIterator<Item = BoxedCondition<'a, QS, DB>>,
    junction: Junction,
) -> Option<BoxedCondition<'a, QS, DB>>
where
    QS: 'a,
    DB: Backend + 'a,
    dsl::And<BoxedCondition<'a, QS, DB>, BoxedCondition<'a, QS, DB>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>>,
    dsl::Or<BoxedCondition<'a, QS, DB>, BoxedCondition<'a, QS, DB>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>>,
{
    conditions
        .into_iter()
        .reduce(|joined, condition| match junction {
            Junction::And => Box::new(joined.and(condition)),
            Junction::Or => Box::new(joined.or(condition)),
        })
}

/// The ordering `column ASC` or `column DESC` for a sort key of the model.
pub fn ordering<'a, C, QS, DB>(column: C, sort_key: SortKey) -> BoxedOrdering<'a, QS, DB>
where
    DB: Backend,
    C: ExpressionMethods,
    dsl::Asc<C>: BoxableExpression<QS, DB, SqlType = NotSelectable> + 'a,
    dsl::Desc<C>: BoxableExpression<QS, DB, SqlType = NotSelectable> + 'a,
{
    match sort_key.direction {
        SortDirectionModel::Ascending => Box::new(column.asc()),
        SortDirectionModel::Descending => Box::new(column.desc()),
    }
}

/// The `LIMIT` and `OFFSET` of page `page` (0 based) of `page_size` rows, for
/// a boxed query's `limit` and `offset`; `None` unless both are requested.
#[must_use]
pub fn page(page: Option<u32>, page_size: Option<u32>) -> Option<(i64, i64)> {
    let (Some(page), Some(page_size)) = (page, page_size) else {
        return None;
    };

    Some((i64::from(page_size), page_offset(page, page_size)))
}

/// What a column takes, at compile time: the [`Value`] variants of its
/// non-null SQL type ([`ValueSqlType::ACCEPTS`]), and whether it may be
/// `NULL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnType {
    /// The variants it takes.
    pub accepts: &'static [ValueKind],
    /// Whether it may be `NULL`.
    pub nullable: bool,
}

impl ColumnType {
    /// A `NOT NULL` column of the SQL type `ST`.
    #[must_use]
    pub const fn not_null<ST: ValueSqlType>() -> Self {
        Self {
            accepts: ST::ACCEPTS,
            nullable: false,
        }
    }
}

/// The Rust value a column whose SQL type is `Self` is set to: the SQL type's
/// Rust type, or an `Option` of it for a nullable column.
#[diagnostic::on_unimplemented(
    message = "queryweave cannot write a domain value to a column of SQL type `{Self}`",
    label = "no `queryweave_sql::diesel::ColumnValue` for this SQL type",
    note = "mark the entity field of a column storing no domain field `#[columns(skip)]`, or implement `ValueSqlType`, `Comparison` and `ColumnValue` for the SQL type"
)]
pub trait ColumnValue {
    /// What the column takes, for the compile-time check that a domain
    /// field's value fits it: [`ColumnType::not_null`] for a `NOT NULL` SQL
    /// type.
    const TYPE: ColumnType;

    /// The Rust type.
    type Rust;

    /// The value as that type; `None` if it does not fit (another type, or a
    /// `NULL` for a `NOT NULL` column).
    fn from_value(value: Value) -> Option<Self::Rust>;
}

impl<ST: ValueSqlType> ColumnValue for Nullable<ST> {
    const TYPE: ColumnType = ColumnType {
        accepts: ST::ACCEPTS,
        nullable: true,
    };

    type Rust = Option<ST::Rust>;

    fn from_value(value: Value) -> Option<Self::Rust> {
        ST::rust(value).ok()
    }
}

/// The Rust value type of the column `C`, by its SQL type.
pub type ValueOf<C> = <<C as Expression>::SqlType as ColumnValue>::Rust;

/// The assignment `column = value` for a merge-patch write of the model.
///
/// The column is the one the entity stores the domain field in; the
/// assignment is one item of a changeset (`Option`s of these, `None` for the
/// unchanged columns).
///
/// # Errors
///
/// [`ValueMismatch`] if the value does not fit the column's SQL type.
#[cfg(feature = "merge-patch")]
pub fn assignment<C>(
    column: C,
    field_value: FieldValue,
) -> Result<dsl::Eq<C, ValueOf<C>>, ValueMismatch>
where
    C: Expression + ExpressionMethods,
    C::SqlType: SqlType + TypedExpressionType + ColumnValue,
    ValueOf<C>: AsExpression<C::SqlType>,
{
    let FieldValue { field, value } = field_value;
    let value = <C::SqlType as ColumnValue>::from_value(value).ok_or(ValueMismatch { field })?;

    Ok(column.eq(value))
}

/// A Diesel column a model's filter conditions can compare: one whose SQL
/// type converts a domain value ([`ValueSqlType`] and [`Comparison`]).
///
/// Blanket-implemented over [`condition`]: the code generated for an entity
/// requires it of every column, so a column of another SQL type is reported
/// where the conditions are written, naming the column.
#[diagnostic::on_unimplemented(
    message = "queryweave cannot filter on the column `{Self}`: its SQL type takes no domain value",
    label = "a column of a SQL type with no `queryweave_sql::diesel::ValueSqlType`",
    note = "mark the entity field of a column storing no domain field `#[columns(skip)]`, or implement `ValueSqlType`, `Comparison` and `ColumnValue` for its SQL type"
)]
pub trait FilterColumn<'a, QS, DB>: Sized {
    /// The [`Value`] variants the column's SQL type takes, for the
    /// compile-time check that a domain field's value fits it.
    const ACCEPTS: &'static [ValueKind];

    /// See [`condition`].
    ///
    /// # Errors
    ///
    /// See [`condition`].
    fn condition(
        self,
        filter_condition: FilterCondition,
    ) -> Result<BoxedCondition<'a, QS, DB>, ValueMismatch>;
}

impl<'a, C, QS, DB> FilterColumn<'a, QS, DB> for C
where
    C: Expression,
    C::SqlType: IntoNullable,
    <C::SqlType as IntoNullable>::Nullable: NonNull,
    NonNullSqlType<C>: Comparison<'a, C, QS, DB>,
{
    const ACCEPTS: &'static [ValueKind] = <NonNullSqlType<C> as ValueSqlType>::ACCEPTS;

    fn condition(
        self,
        filter_condition: FilterCondition,
    ) -> Result<BoxedCondition<'a, QS, DB>, ValueMismatch> {
        condition(self, filter_condition)
    }
}

/// A Diesel column a model's sort keys can order by.
///
/// Blanket-implemented over [`ordering`], for every column of the table.
#[diagnostic::on_unimplemented(
    message = "queryweave cannot order by the column `{Self}`",
    note = "mark the entity field of a column storing no domain field `#[columns(skip)]`"
)]
pub trait OrderColumn<'a, QS, DB>: Sized {
    /// See [`ordering`].
    fn ordering(self, sort_key: SortKey) -> BoxedOrdering<'a, QS, DB>;
}

impl<'a, C, QS, DB> OrderColumn<'a, QS, DB> for C
where
    DB: Backend,
    C: ExpressionMethods,
    dsl::Asc<C>: BoxableExpression<QS, DB, SqlType = NotSelectable> + 'a,
    dsl::Desc<C>: BoxableExpression<QS, DB, SqlType = NotSelectable> + 'a,
{
    fn ordering(self, sort_key: SortKey) -> BoxedOrdering<'a, QS, DB> {
        ordering(self, sort_key)
    }
}

/// A Diesel column a model's merge-patch writes can set: one whose SQL type
/// converts a domain value ([`ColumnValue`]).
///
/// Blanket-implemented over [`assignment`].
#[cfg(feature = "merge-patch")]
#[diagnostic::on_unimplemented(
    message = "queryweave cannot write the column `{Self}`: its SQL type takes no domain value",
    label = "a column of a SQL type with no `queryweave_sql::diesel::ColumnValue`",
    note = "mark the entity field of a column storing no domain field `#[columns(skip)]`, or implement `ValueSqlType`, `Comparison` and `ColumnValue` for its SQL type"
)]
pub trait AssignColumn: Sized {
    /// What the column takes, for the compile-time check that a domain
    /// field's value fits it ([`ColumnValue::TYPE`]).
    const TYPE: ColumnType;

    /// The assignment `column = value`.
    type Assignment;

    /// See [`assignment`].
    ///
    /// # Errors
    ///
    /// See [`assignment`].
    fn assignment(self, field_value: FieldValue) -> Result<Self::Assignment, ValueMismatch>;
}

#[cfg(feature = "merge-patch")]
impl<C> AssignColumn for C
where
    C: Expression + ExpressionMethods,
    C::SqlType: SqlType + TypedExpressionType + ColumnValue,
    ValueOf<C>: AsExpression<C::SqlType>,
{
    const TYPE: ColumnType = <C::SqlType as ColumnValue>::TYPE;

    type Assignment = dsl::Eq<C, ValueOf<C>>;

    fn assignment(self, field_value: FieldValue) -> Result<Self::Assignment, ValueMismatch> {
        assignment(self, field_value)
    }
}

/// The assignment `column = value` of the column `C`: one slot of an entity's
/// changeset (an `Option` of it, `None` when unchanged).
#[cfg(feature = "merge-patch")]
pub type Assignment<C> = <C as AssignColumn>::Assignment;

/// A backend whose conditions on the table `QS` can be joined: every Diesel
/// backend.
///
/// [`join`] as a method, so the code generated for an entity needs this one
/// bound rather than [`join`]'s.
pub trait JoinConditions<'a, QS>: Backend + Sized {
    /// See [`join`].
    fn join(
        conditions: impl IntoIterator<Item = BoxedCondition<'a, QS, Self>>,
        junction: Junction,
    ) -> Option<BoxedCondition<'a, QS, Self>>;
}

impl<'a, QS, DB> JoinConditions<'a, QS> for DB
where
    QS: 'a,
    DB: Backend + 'a,
    dsl::And<BoxedCondition<'a, QS, DB>, BoxedCondition<'a, QS, DB>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>>,
    dsl::Or<BoxedCondition<'a, QS, DB>, BoxedCondition<'a, QS, DB>>:
        BoxableExpression<QS, DB, SqlType = Nullable<Bool>>,
{
    fn join(
        conditions: impl IntoIterator<Item = BoxedCondition<'a, QS, Self>>,
        junction: Junction,
    ) -> Option<BoxedCondition<'a, QS, Self>> {
        join(conditions, junction)
    }
}

/// A Diesel entity: one whose columns the model's data can name, implemented
/// by `#[derive(Columns)]` for an entity with `#[diesel(table_name = ..)]`.
///
/// Its methods are the entity's `diesel_conditions` and `diesel_order_by` as
/// a trait, so the code generated for the wrappers (`AppendToQuery`,
/// `AppendToQueryWithSeparator`), which knows the entity but not its table,
/// can name them. `DB` is the backend, `'a` how long the boxed expressions
/// live.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a Diesel entity for `{DB}`",
    label = "no `queryweave_sql::diesel::Entity` for this entity",
    note = "derive `Columns` on an entity with `#[diesel(table_name = ...)]`, every column of a SQL type queryweave converts (or `#[columns(skip)]`)"
)]
pub trait Entity<'a, DB> {
    /// The entity's table (`table_name::table`).
    type Table;

    /// The conditions of the model `M`, joined by `junction`, as one
    /// condition on the table; `None` when there are none.
    ///
    /// # Errors
    ///
    /// [`ValueMismatch`] if a condition's value does not fit its column's SQL
    /// type.
    fn conditions<M: PersistedFields>(
        conditions: Conditions<M>,
        junction: Junction,
    ) -> Result<Option<BoxedCondition<'a, Self::Table, DB>>, ValueMismatch>;

    /// The sort keys of the model `M`, in order, as orderings of the table.
    fn order_by<M: PersistedFields>(
        sort_keys: SortKeys<M>,
    ) -> Vec<BoxedOrdering<'a, Self::Table, DB>>;
}

/// A boxed query on the table `QS` (`table.into_boxed()`, after any `select`
/// or `filter`): one taking conditions, orderings, a limit and an offset as
/// the same type, which [`apply`] adds.
///
/// Blanket-implemented for every query that does: Diesel's boxed select
/// statements.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a boxed query of the entity's table",
    label = "not a boxed query",
    note = "box the query first, with `.into_boxed()`"
)]
pub trait BoxedQuery<'a, QS, DB>:
    FilterDsl<BoxedCondition<'a, QS, DB>, Output = Self>
    + ThenOrderDsl<BoxedOrdering<'a, QS, DB>, Output = Self>
    + LimitDsl<Output = Self>
    + OffsetDsl<Output = Self>
{
}

impl<'a, QS, DB, Q> BoxedQuery<'a, QS, DB> for Q where
    Q: FilterDsl<BoxedCondition<'a, QS, DB>, Output = Self>
        + ThenOrderDsl<BoxedOrdering<'a, QS, DB>, Output = Self>
        + LimitDsl<Output = Self>
        + OffsetDsl<Output = Self>
{
}

/// The boxed query `query` with the condition (its `filter`, so `AND`ed to
/// the query's own), the orderings after its own (`then_order_by`), and the
/// page (`limit` and `offset`, from [`page`]).
#[must_use]
pub fn apply<'a, Q, QS, DB>(
    query: Q,
    condition: Option<BoxedCondition<'a, QS, DB>>,
    orderings: Vec<BoxedOrdering<'a, QS, DB>>,
    page: Option<(i64, i64)>,
) -> Q
where
    Q: BoxedQuery<'a, QS, DB>,
{
    let mut query = match condition {
        Some(condition) => FilterDsl::filter(query, condition),
        None => query,
    };
    for ordering in orderings {
        query = ThenOrderDsl::then_order_by(query, ordering);
    }
    match page {
        Some((limit, offset)) => OffsetDsl::offset(LimitDsl::limit(query, limit), offset),
        None => query,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "merge-patch")]
    use super::assignment;
    use super::{ValueMismatch, apply, condition, join, ordering, page};
    use crate::Junction;
    use diesel::mysql::Mysql;
    use diesel::pg::Pg;
    use diesel::prelude::*;
    use diesel::sqlite::Sqlite;
    use pretty_assertions::assert_eq;
    #[cfg(feature = "merge-patch")]
    use queryweave::FieldValue;
    use queryweave::{FilterCondition, FilterOperator, SortDirectionModel, SortKey, Value};
    use rstest::rstest;

    diesel::table! {
        orders (id) {
            id -> Integer,
            number -> Text,
            client -> Nullable<Text>,
            capacity -> Integer,
            value -> Double,
            delivered_at -> Timestamp,
        }
    }

    fn filter(field: &'static str, operator: FilterOperator, value: Value) -> FilterCondition {
        FilterCondition {
            field,
            operator,
            value,
        }
    }

    /// A select of the ids, filtered by a condition on `capacity`, rendered
    /// for each backend.
    #[test]
    fn every_backend_renders_the_comparison() {
        let less_than = || filter("capacity", FilterOperator::LessThan, Value::I32(Some(9)));

        let pg = orders::table
            .select(orders::id)
            .into_boxed::<Pg>()
            .filter(condition(orders::capacity, less_than()).unwrap());
        assert_eq!(
            diesel::debug_query::<Pg, _>(&pg).to_string(),
            r#"SELECT "orders"."id" FROM "orders" WHERE ("orders"."capacity" < $1) -- binds: [Some(9)]"#
        );

        let mysql = orders::table
            .select(orders::id)
            .into_boxed::<Mysql>()
            .filter(condition(orders::capacity, less_than()).unwrap());
        assert_eq!(
            diesel::debug_query::<Mysql, _>(&mysql).to_string(),
            "SELECT `orders`.`id` FROM `orders` WHERE (`orders`.`capacity` < ?) -- binds: [Some(9)]"
        );

        let sqlite = orders::table
            .select(orders::id)
            .into_boxed::<Sqlite>()
            .filter(condition(orders::capacity, less_than()).unwrap());
        assert_eq!(
            diesel::debug_query::<Sqlite, _>(&sqlite).to_string(),
            "SELECT `orders`.`id` FROM `orders` WHERE (`orders`.`capacity` < ?) -- binds: [Some(9)]"
        );
    }

    /// The Postgres `WHERE` of a select of the ids with the condition.
    fn pg_where(condition: super::BoxedCondition<'static, orders::table, Pg>) -> String {
        let query = orders::table
            .select(orders::id)
            .into_boxed::<Pg>()
            .filter(condition);
        diesel::debug_query::<Pg, _>(&query).to_string()
    }

    // The operator selects the comparison; the `LIKE` ones bind an escaped
    // pattern and the escape character, nullable column or not.
    #[rstest]
    #[case(
        FilterOperator::LessThan,
        r#""orders"."client" < $1) -- binds: [Some("50%")]"#
    )]
    #[case(
        FilterOperator::Equal,
        r#""orders"."client" = $1) -- binds: [Some("50%")]"#
    )]
    #[case(
        FilterOperator::GreaterThan,
        r#""orders"."client" > $1) -- binds: [Some("50%")]"#
    )]
    #[case(
        FilterOperator::StartsWith,
        r#""orders"."client" LIKE $1 ESCAPE $2) -- binds: ["50!%%", "!"]"#
    )]
    #[case(
        FilterOperator::Contains,
        r#""orders"."client" LIKE $1 ESCAPE $2) -- binds: ["%50!%%", "!"]"#
    )]
    #[case(
        FilterOperator::EndsWith,
        r#""orders"."client" LIKE $1 ESCAPE $2) -- binds: ["%50!%", "!"]"#
    )]
    fn the_operator_selects_the_comparison(
        #[case] operator: FilterOperator,
        #[case] expected: &str,
    ) {
        let filter_condition = filter("client", operator, Value::Text(Some(String::from("50%"))));

        assert_eq!(
            pg_where(condition(orders::client, filter_condition).unwrap()),
            format!(r#"SELECT "orders"."id" FROM "orders" WHERE ({expected}"#)
        );
    }

    // A narrower value widens to its column's type; a `NULL` binds as one.
    #[rstest]
    #[case(
        condition(
            orders::capacity,
            filter("capacity", FilterOperator::Equal, Value::I16(Some(3)))
        ),
        r#""orders"."capacity" = $1) -- binds: [Some(3)]"#
    )]
    #[case(
        condition(
            orders::value,
            filter("value", FilterOperator::GreaterThan, Value::F32(Some(1.5)))
        ),
        r#""orders"."value" > $1) -- binds: [Some(1.5)]"#
    )]
    #[case(
        condition(
            orders::number,
            filter("number", FilterOperator::Equal, Value::Text(None))
        ),
        r#""orders"."number" = $1) -- binds: [None]"#
    )]
    fn values_bind_with_the_columns_type(
        #[case] condition: Result<super::BoxedCondition<'static, orders::table, Pg>, ValueMismatch>,
        #[case] expected: &str,
    ) {
        assert_eq!(
            pg_where(condition.unwrap()),
            format!(r#"SELECT "orders"."id" FROM "orders" WHERE ({expected}"#)
        );
    }

    // A value of another type, or `LIKE` on a column that is not text, is a
    // mismatch naming the domain field.
    #[rstest]
    #[case(FilterOperator::Equal, Value::Text(Some(String::from("3"))))]
    #[case(FilterOperator::Equal, Value::I64(Some(3)))]
    #[case(FilterOperator::Contains, Value::I32(Some(3)))]
    fn a_value_that_does_not_fit_is_a_mismatch(
        #[case] operator: FilterOperator,
        #[case] value: Value,
    ) {
        let result: Result<super::BoxedCondition<'static, orders::table, Pg>, _> =
            condition(orders::capacity, filter("amount", operator, value));

        assert_eq!(result.err(), Some(ValueMismatch { field: "amount" }));
    }

    // Conditions are joined by the junction; without any there is none.
    #[rstest]
    #[case(Junction::And, "AND")]
    #[case(Junction::Or, "OR")]
    fn conditions_are_joined_by_the_junction(#[case] junction: Junction, #[case] keyword: &str) {
        let conditions = [
            filter("capacity", FilterOperator::GreaterThan, Value::I32(Some(1))),
            filter("capacity", FilterOperator::LessThan, Value::I32(Some(9))),
        ]
        .map(|filter_condition| condition(orders::capacity, filter_condition).unwrap());

        assert_eq!(
            pg_where(join(conditions, junction).unwrap()),
            format!(
                r#"SELECT "orders"."id" FROM "orders" WHERE (("orders"."capacity" > $1) {keyword} ("orders"."capacity" < $2)) -- binds: [Some(1), Some(9)]"#
            )
        );
        assert!(join::<orders::table, Pg>([], junction).is_none());
    }

    // Sort keys order in their direction, a page is a limit and an offset.
    #[test]
    fn orderings_and_pages_apply_to_a_boxed_query() {
        let (limit, offset) = page(Some(2), Some(10)).unwrap();
        let query = orders::table
            .select(orders::id)
            .into_boxed::<Pg>()
            .then_order_by(ordering(
                orders::delivered_at,
                SortKey {
                    field: "delivery_date",
                    direction: SortDirectionModel::Descending,
                },
            ))
            .then_order_by(ordering(
                orders::number,
                SortKey {
                    field: "number",
                    direction: SortDirectionModel::Ascending,
                },
            ))
            .limit(limit)
            .offset(offset);

        assert_eq!(
            diesel::debug_query::<Pg, _>(&query).to_string(),
            r#"SELECT "orders"."id" FROM "orders" ORDER BY "orders"."delivered_at" DESC, "orders"."number" ASC LIMIT $1 OFFSET $2 -- binds: [10, 20]"#
        );
        assert_eq!(page(None, Some(10)), None);
        assert_eq!(page(Some(1), None), None);
    }

    // `apply` adds a condition after the query's own, orderings after its
    // own, and a page; nothing it is not given.
    #[test]
    fn apply_adds_the_condition_orderings_and_page() {
        let capacity = filter("capacity", FilterOperator::LessThan, Value::I32(Some(9)));
        let number = SortKey {
            field: "number",
            direction: SortDirectionModel::Ascending,
        };
        let query = orders::table
            .select(orders::id)
            .into_boxed::<Pg>()
            .filter(orders::id.gt(0))
            .order_by(orders::id);

        let query = apply(
            query,
            Some(condition(orders::capacity, capacity).unwrap()),
            vec![ordering(orders::number, number)],
            page(Some(1), Some(10)),
        );

        assert_eq!(
            diesel::debug_query::<Pg, _>(&query).to_string(),
            r#"SELECT "orders"."id" FROM "orders" WHERE (("orders"."id" > $1) AND ("orders"."capacity" < $2)) ORDER BY "orders"."id", "orders"."number" ASC LIMIT $3 OFFSET $4 -- binds: [0, Some(9), 10, 10]"#
        );

        let untouched = apply(
            orders::table.select(orders::id).into_boxed::<Pg>(),
            None,
            Vec::new(),
            None,
        );
        assert_eq!(
            diesel::debug_query::<Pg, _>(&untouched).to_string(),
            r#"SELECT "orders"."id" FROM "orders" -- binds: []"#
        );
    }

    // Assignments set each column with its type: a nullable column to `NULL`,
    // the unchanged columns left out of the changeset.
    #[cfg(feature = "merge-patch")]
    #[test]
    fn assignments_make_a_changeset() {
        let client = FieldValue {
            field: "client",
            value: Value::Text(None),
        };
        let capacity = FieldValue {
            field: "capacity",
            value: Value::I16(Some(4)),
        };
        let changeset = (
            Some(assignment(orders::client, client).unwrap()),
            Some(assignment(orders::capacity, capacity).unwrap()),
            None::<diesel::dsl::Eq<orders::number, String>>,
        );
        let update = diesel::update(orders::table).set(changeset);

        assert_eq!(
            diesel::debug_query::<Pg, _>(&update).to_string(),
            r#"UPDATE "orders" SET "client" = $1, "capacity" = $2 -- binds: [None, 4]"#
        );
    }

    // A `NOT NULL` column cannot be set to `NULL`, nor to another type.
    #[cfg(feature = "merge-patch")]
    #[rstest]
    #[case(Value::I32(None))]
    #[case(Value::Text(Some(String::from("4"))))]
    fn an_assignment_that_does_not_fit_is_a_mismatch(#[case] value: Value) {
        let field_value = FieldValue {
            field: "amount",
            value,
        };

        assert_eq!(
            assignment(orders::capacity, field_value).err(),
            Some(ValueMismatch { field: "amount" })
        );
    }

    #[test]
    fn the_mismatch_names_the_field() {
        assert_eq!(
            ValueMismatch { field: "amount" }.to_string(),
            "the value of the domain field `amount` does not fit the SQL type of its column"
        );
    }

    fn utc() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_767_268_800, 0).unwrap()
    }

    // Each SQL type takes its own variant and the narrower ones, without loss
    // (a UTC date-time as its naive UTC time); a `NULL` stays one, and any
    // other variant is handed back.
    #[test]
    fn each_sql_type_takes_its_values() {
        use super::ValueSqlType;
        use diesel::sql_types::{
            BigInt, Bool, Date, Double, Float, Integer, SmallInt, Text, Time, Timestamp,
        };

        assert_eq!(Bool::rust(Value::Bool(Some(true))), Ok(Some(true)));
        assert_eq!(Bool::rust(Value::I32(Some(1))), Err(Value::I32(Some(1))));

        assert_eq!(SmallInt::rust(Value::I16(Some(1))), Ok(Some(1)));
        assert_eq!(
            SmallInt::rust(Value::I32(Some(1))),
            Err(Value::I32(Some(1)))
        );

        assert_eq!(Integer::rust(Value::I32(Some(1))), Ok(Some(1)));
        assert_eq!(Integer::rust(Value::I16(Some(2))), Ok(Some(2)));
        assert_eq!(Integer::rust(Value::I64(Some(3))), Err(Value::I64(Some(3))));

        assert_eq!(BigInt::rust(Value::I64(Some(1))), Ok(Some(1)));
        assert_eq!(BigInt::rust(Value::I32(Some(2))), Ok(Some(2)));
        assert_eq!(BigInt::rust(Value::I16(Some(3))), Ok(Some(3)));

        assert_eq!(Float::rust(Value::F32(Some(1.5))), Ok(Some(1.5)));
        assert_eq!(
            Float::rust(Value::F64(Some(1.5))),
            Err(Value::F64(Some(1.5)))
        );

        assert_eq!(Double::rust(Value::F64(Some(1.5))), Ok(Some(1.5)));
        assert_eq!(Double::rust(Value::F32(Some(2.5))), Ok(Some(2.5)));

        assert_eq!(
            Text::rust(Value::Text(Some(String::from("a")))),
            Ok(Some(String::from("a")))
        );
        assert_eq!(Text::rust(Value::Text(None)), Ok(None));
        assert_eq!(Text::rust(Value::I32(Some(1))), Err(Value::I32(Some(1))));

        assert_eq!(
            Timestamp::rust(Value::NaiveDateTime(Some(utc().naive_utc()))),
            Ok(Some(utc().naive_utc()))
        );
        assert_eq!(
            Timestamp::rust(Value::DateTimeUtc(Some(utc()))),
            Ok(Some(utc().naive_utc()))
        );

        assert_eq!(
            Date::rust(Value::NaiveDate(Some(utc().date_naive()))),
            Ok(Some(utc().date_naive()))
        );
        assert_eq!(
            Date::rust(Value::DateTimeUtc(Some(utc()))),
            Err(Value::DateTimeUtc(Some(utc())))
        );

        assert_eq!(
            Time::rust(Value::NaiveTime(Some(utc().time()))),
            Ok(Some(utc().time()))
        );
    }

    // Each backend's own date-time type takes a UTC date-time (MySQL's, which
    // has no time zone, as its naive UTC time).
    #[cfg(feature = "diesel-postgres")]
    #[test]
    fn postgres_timestamptz_takes_a_utc_date_time() {
        use super::ValueSqlType;
        use diesel::sql_types::Timestamptz;

        assert_eq!(
            Timestamptz::rust(Value::DateTimeUtc(Some(utc()))),
            Ok(Some(utc()))
        );
        assert_eq!(
            Timestamptz::rust(Value::NaiveDateTime(Some(utc().naive_utc()))),
            Err(Value::NaiveDateTime(Some(utc().naive_utc())))
        );
    }

    #[cfg(feature = "diesel-sqlite")]
    #[test]
    fn sqlite_timestamptz_takes_a_utc_date_time() {
        use super::ValueSqlType;
        use diesel::sql_types::TimestamptzSqlite;

        assert_eq!(
            TimestamptzSqlite::rust(Value::DateTimeUtc(Some(utc()))),
            Ok(Some(utc()))
        );
    }

    #[cfg(feature = "diesel-mysql")]
    #[test]
    fn mysql_datetime_takes_a_naive_or_utc_date_time() {
        use super::ValueSqlType;
        use diesel::sql_types::Datetime;

        assert_eq!(
            Datetime::rust(Value::NaiveDateTime(Some(utc().naive_utc()))),
            Ok(Some(utc().naive_utc()))
        );
        assert_eq!(
            Datetime::rust(Value::DateTimeUtc(Some(utc()))),
            Ok(Some(utc().naive_utc()))
        );
    }

    // A column's value: a `NOT NULL` column takes no `NULL`, a nullable one
    // does; a value of another type fits neither.
    #[test]
    fn a_columns_value_follows_its_nullability() {
        use super::ColumnValue;
        use diesel::sql_types::{Integer, Nullable};

        assert_eq!(Integer::from_value(Value::I32(Some(1))), Some(1));
        assert_eq!(Integer::from_value(Value::I32(None)), None);
        assert_eq!(
            Nullable::<Integer>::from_value(Value::I32(None)),
            Some(None)
        );
        assert_eq!(
            Nullable::<Integer>::from_value(Value::I32(Some(1))),
            Some(Some(1))
        );
        assert_eq!(Nullable::<Integer>::from_value(Value::Text(None)), None);
    }

    /// A `NULL` of every `Value` variant.
    fn nulls() -> [Value; 11] {
        [
            Value::Bool(None),
            Value::I16(None),
            Value::I32(None),
            Value::I64(None),
            Value::F32(None),
            Value::F64(None),
            Value::Text(None),
            Value::DateTimeUtc(None),
            Value::NaiveDateTime(None),
            Value::NaiveDate(None),
            Value::NaiveTime(None),
        ]
    }

    /// `ST::ACCEPTS` lists exactly the variants `ST::rust` takes.
    fn assert_accepts<ST: super::ValueSqlType>() {
        for value in nulls() {
            let kind = value.kind();
            assert_eq!(
                ST::rust(value).is_ok(),
                ST::ACCEPTS.contains(&kind),
                "{kind:?} for {}",
                std::any::type_name::<ST>()
            );
        }
    }

    // The compile-time check and the conversion agree on what a SQL type
    // takes.
    #[test]
    fn each_sql_type_accepts_the_variants_it_converts() {
        use diesel::sql_types::{
            BigInt, Bool, Date, Double, Float, Integer, SmallInt, Text, Time, Timestamp,
        };

        assert_accepts::<Bool>();
        assert_accepts::<SmallInt>();
        assert_accepts::<Integer>();
        assert_accepts::<BigInt>();
        assert_accepts::<Float>();
        assert_accepts::<Double>();
        assert_accepts::<Text>();
        assert_accepts::<Timestamp>();
        assert_accepts::<Date>();
        assert_accepts::<Time>();
        #[cfg(feature = "diesel-postgres")]
        assert_accepts::<diesel::sql_types::Timestamptz>();
        #[cfg(feature = "diesel-sqlite")]
        assert_accepts::<diesel::sql_types::TimestamptzSqlite>();
        #[cfg(feature = "diesel-mysql")]
        assert_accepts::<diesel::sql_types::Datetime>();
    }

    // A column's type: its SQL type's variants, nullable or not.
    #[test]
    fn a_columns_type_follows_its_sql_type() {
        use super::{ColumnType, ColumnValue, FilterColumn};
        use queryweave::ValueKind;

        assert_eq!(
            <orders::capacity as FilterColumn<'static, orders::table, Pg>>::ACCEPTS,
            &[ValueKind::I32, ValueKind::I16]
        );
        assert_eq!(
            <diesel::sql_types::Integer as ColumnValue>::TYPE,
            ColumnType {
                accepts: &[ValueKind::I32, ValueKind::I16],
                nullable: false,
            }
        );
        assert_eq!(
            <diesel::sql_types::Nullable<diesel::sql_types::Text> as ColumnValue>::TYPE,
            ColumnType {
                accepts: &[ValueKind::Text],
                nullable: true,
            }
        );
    }
}
