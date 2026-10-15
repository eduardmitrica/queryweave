//! The check that each domain field's value fits its column's type in a
//! `SeaORM` entity (or any table described by sea-query's `ColumnType`).
//!
//! sea-query binds whatever value it is given, and the database decides:
//! Postgres rejects a value of another type when the query runs, MySQL and
//! SQLite convert it, so a mismatch may silently match the wrong rows. The
//! domain derives publish each field's value type
//! (`PersistedFields::FIELD_TYPES`), and `SeaORM` each column's
//! (`ColumnTrait::def`), so [`check_column_types`] compares the two, once,
//! from a test or at startup.

use crate::Columns;
use queryweave::{PersistedFields, ValueKind, ValueType};
use sea_query::ColumnType;

/// A domain field whose value does not fit its column; see
/// [`check_column_types`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnTypeMismatch {
    /// The entity's type name.
    pub entity: &'static str,
    /// The domain field.
    pub field: &'static str,
    /// The column storing it.
    pub column: &'static str,
    /// What does not fit.
    pub reason: MismatchReason,
}

/// Why a domain field's value does not fit its column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MismatchReason {
    /// The value is of a type the column does not take.
    Kind {
        /// The value's type.
        value: ValueKind,
        /// What the column takes (empty: no domain value).
        accepts: &'static [ValueKind],
        /// The column's type, as sea-query names it.
        column_type: String,
    },
    /// The field may be `None`, and the column is `NOT NULL`.
    Nullable,
    /// The column is not among the columns given.
    MissingColumn,
}

impl std::fmt::Display for ColumnTypeMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            entity,
            field,
            column,
            reason,
        } = self;
        match reason {
            MismatchReason::Kind {
                value,
                accepts,
                column_type,
            } => {
                write!(
                    formatter,
                    "the value of domain field `{field}` (`{}`) does not fit the column `{column}` of `{entity}`, which ",
                    value.rust_type()
                )?;
                if accepts.is_empty() {
                    write!(formatter, "takes no domain value (`{column_type}`)")?;
                } else {
                    let accepts = accepts
                        .iter()
                        .map(|kind| format!("`{}`", kind.rust_type()))
                        .collect::<Vec<_>>()
                        .join(" or ");
                    write!(formatter, "takes {accepts}")?;
                }
                write!(
                    formatter,
                    ": make the field's type and the column's type in the entity agree"
                )
            },
            MismatchReason::Nullable => write!(
                formatter,
                "domain field `{field}` may be `None`, but the column `{column}` of `{entity}` is `NOT NULL`: make the column nullable in the entity, or the field not optional"
            ),
            MismatchReason::MissingColumn => write!(
                formatter,
                "the column `{column}` of `{entity}` (storing domain field `{field}`) is not among the columns given"
            ),
        }
    }
}

impl std::error::Error for ColumnTypeMismatch {}

const INTEGERS: &[ValueKind] = &[ValueKind::I16, ValueKind::I32, ValueKind::I64];
const NUMBERS: &[ValueKind] = &[
    ValueKind::I16,
    ValueKind::I32,
    ValueKind::I64,
    ValueKind::F32,
    ValueKind::F64,
];
const FLOATS: &[ValueKind] = &[ValueKind::F32, ValueKind::F64];

/// The value types a column of `column_type` takes; `None` for a custom type,
/// which is not checked.
///
/// Those sea-query binds as a type the database stores in it without loss of
/// meaning: any integer in any integer column (out-of-range values are the
/// database's to reject, as with sqlx). A UTC date-time only in a column with
/// a time zone: Postgres would convert a column without one to compare or
/// store it by the session's time zone, so results would depend on the
/// server's settings.
#[must_use]
pub const fn accepted_kinds(column_type: &ColumnType) -> Option<&'static [ValueKind]> {
    Some(match column_type {
        ColumnType::Boolean => &[ValueKind::Bool],
        ColumnType::TinyInteger
        | ColumnType::SmallInteger
        | ColumnType::Integer
        | ColumnType::BigInteger
        | ColumnType::TinyUnsigned
        | ColumnType::SmallUnsigned
        | ColumnType::Unsigned
        | ColumnType::BigUnsigned
        | ColumnType::Year => INTEGERS,
        ColumnType::Float | ColumnType::Double => FLOATS,
        ColumnType::Decimal(_) | ColumnType::Money(_) => NUMBERS,
        ColumnType::Char(_) | ColumnType::String(_) | ColumnType::Text => &[ValueKind::Text],
        ColumnType::TimestampWithTimeZone => &[ValueKind::DateTimeUtc],
        ColumnType::DateTime | ColumnType::Timestamp => &[ValueKind::NaiveDateTime],
        ColumnType::Date => &[ValueKind::NaiveDate],
        ColumnType::Time => &[ValueKind::NaiveTime],
        ColumnType::Custom(_) => return None,
        _ => &[],
    })
}

/// Check that the value of every field of the model `M` fits its column on
/// the entity `C`.
///
/// `columns` describes the entity's columns: `(column name, column type,
/// nullable)` for each, e.g. from `SeaORM`'s `Column::iter()` and
/// `ColumnTrait::def()`.
///
/// A field that may be `None` (a `MergePatch` model's optional field) needs a
/// nullable column. A model without value types (a hand-written
/// `PersistedFields`) passes unchecked, as does a column of a custom type.
///
/// # Examples
///
/// ```rust,ignore
/// use sea_orm::{ColumnTrait, IdenStatic, Iterable};
///
/// let columns = order::Column::iter().map(|column| {
///     let def = column.def();
///     (column.as_str(), def.get_column_type().clone(), def.is_null())
/// });
/// queryweave_sql::sea_query::check_column_types::<order::Model, OrdersFilterOptionsModel>(
///     columns,
/// )?;
/// ```
///
/// # Errors
///
/// The first field whose value does not fit its column, or whose column is
/// not among `columns`.
pub fn check_column_types<'a, C, M>(
    columns: impl IntoIterator<Item = (&'a str, ColumnType, bool)>,
) -> Result<(), ColumnTypeMismatch>
where
    C: Columns + ?Sized,
    M: PersistedFields,
{
    if M::FIELD_TYPES.len() != M::FIELD_NAMES.len() {
        return Ok(());
    }
    let columns: Vec<_> = columns.into_iter().collect();

    for (&field, value) in M::FIELD_NAMES.iter().zip(M::FIELD_TYPES) {
        // Every field has a column (the SQL helpers check it at compile time).
        let Some(column) = C::column(field) else {
            continue;
        };
        let mismatch = |reason| ColumnTypeMismatch {
            entity: C::NAME,
            field,
            column,
            reason,
        };
        let Some((_, column_type, nullable)) = columns.iter().find(|(name, _, _)| *name == column)
        else {
            return Err(mismatch(MismatchReason::MissingColumn));
        };
        if let Some(reason) = value_mismatch(*value, column_type, *nullable) {
            return Err(mismatch(reason));
        }
    }

    Ok(())
}

/// Why a value of `value`'s type does not fit a column, if it does not.
fn value_mismatch(
    value: ValueType,
    column_type: &ColumnType,
    nullable: bool,
) -> Option<MismatchReason> {
    let accepts = accepted_kinds(column_type)?;
    if !accepts.contains(&value.kind) {
        return Some(MismatchReason::Kind {
            value: value.kind,
            accepts,
            column_type: format!("{column_type:?}"),
        });
    }
    if value.nullable && !nullable {
        return Some(MismatchReason::Nullable);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{ColumnTypeMismatch, MismatchReason, accepted_kinds, check_column_types};
    use crate::Columns;
    use pretty_assertions::assert_eq;
    use queryweave::{PersistedFields, ValueKind, ValueType};
    use rstest::rstest;
    use sea_query::{ColumnType, StringLen};

    struct Order;

    impl Columns for Order {
        const NAME: &'static str = "Model";
        const COLUMNS: &'static [(&'static str, &'static str)] =
            &[("quantity", "quantity"), ("delivery_date", "delivered_at")];
    }

    struct Changes;

    impl PersistedFields for Changes {
        const FIELD_NAMES: &'static [&'static str] = &["quantity", "delivery_date"];
        const FIELD_TYPES: &'static [ValueType] = &[
            ValueType {
                kind: ValueKind::I16,
                nullable: false,
            },
            ValueType {
                kind: ValueKind::DateTimeUtc,
                nullable: true,
            },
        ];
    }

    struct HandWritten;

    impl PersistedFields for HandWritten {
        const FIELD_NAMES: &'static [&'static str] = &["quantity"];
    }

    fn check<M: PersistedFields>(
        quantity: ColumnType,
        delivered_at_nullable: bool,
    ) -> Result<(), ColumnTypeMismatch> {
        check_column_types::<Order, M>([
            ("id", ColumnType::Uuid, false),
            ("quantity", quantity, false),
            (
                "delivered_at",
                ColumnType::TimestampWithTimeZone,
                delivered_at_nullable,
            ),
        ])
    }

    #[test]
    fn values_of_a_type_the_columns_take_fit() {
        assert_eq!(check::<Changes>(ColumnType::Integer, true), Ok(()));
        assert_eq!(check::<Changes>(ColumnType::TinyUnsigned, true), Ok(()));
    }

    #[test]
    fn a_value_of_another_type_names_the_field_column_and_types() {
        let mismatch = check::<Changes>(ColumnType::String(StringLen::None), true).unwrap_err();

        assert_eq!(
            mismatch.reason,
            MismatchReason::Kind {
                value: ValueKind::I16,
                accepts: &[ValueKind::Text],
                column_type: String::from("String(None)"),
            }
        );
        assert_eq!(
            mismatch.to_string(),
            "the value of domain field `quantity` (`i16`) does not fit the column `quantity` of `Model`, which takes `String`: make the field's type and the column's type in the entity agree"
        );
    }

    #[test]
    fn a_column_taking_no_domain_value_says_its_type() {
        let mismatch = check::<Changes>(ColumnType::Json, true).unwrap_err();

        assert_eq!(
            mismatch.to_string(),
            "the value of domain field `quantity` (`i16`) does not fit the column `quantity` of `Model`, which takes no domain value (`Json`): make the field's type and the column's type in the entity agree"
        );
    }

    #[test]
    fn a_nullable_value_needs_a_nullable_column() {
        let mismatch = check::<Changes>(ColumnType::Integer, false).unwrap_err();

        assert_eq!(mismatch.reason, MismatchReason::Nullable);
        assert_eq!(
            mismatch.to_string(),
            "domain field `delivery_date` may be `None`, but the column `delivered_at` of `Model` is `NOT NULL`: make the column nullable in the entity, or the field not optional"
        );
    }

    #[test]
    fn a_column_not_given_is_reported() {
        let mismatch =
            check_column_types::<Order, Changes>([("quantity", ColumnType::Integer, false)])
                .unwrap_err();

        assert_eq!(
            mismatch.to_string(),
            "the column `delivered_at` of `Model` (storing domain field `delivery_date`) is not among the columns given"
        );
    }

    // A hand-written `PersistedFields` has no value types: nothing checked.
    #[test]
    fn a_model_without_value_types_is_not_checked() {
        assert_eq!(check::<HandWritten>(ColumnType::Json, false), Ok(()));
    }

    #[rstest]
    #[case(ColumnType::Boolean, ValueKind::Bool, true)]
    #[case(ColumnType::BigInteger, ValueKind::I16, true)]
    #[case(ColumnType::Integer, ValueKind::F64, false)]
    #[case(ColumnType::Double, ValueKind::F32, true)]
    #[case(ColumnType::Decimal(None), ValueKind::I64, true)]
    #[case(ColumnType::Text, ValueKind::Text, true)]
    #[case(ColumnType::Char(None), ValueKind::I32, false)]
    #[case(ColumnType::TimestampWithTimeZone, ValueKind::DateTimeUtc, true)]
    #[case(ColumnType::TimestampWithTimeZone, ValueKind::NaiveDateTime, false)]
    #[case(ColumnType::DateTime, ValueKind::NaiveDateTime, true)]
    #[case(ColumnType::DateTime, ValueKind::DateTimeUtc, false)]
    #[case(ColumnType::Timestamp, ValueKind::DateTimeUtc, false)]
    #[case(ColumnType::Date, ValueKind::NaiveDate, true)]
    #[case(ColumnType::Time, ValueKind::NaiveTime, true)]
    #[case(ColumnType::Uuid, ValueKind::Text, false)]
    fn each_column_type_takes_its_values(
        #[case] column_type: ColumnType,
        #[case] kind: ValueKind,
        #[case] takes: bool,
    ) {
        assert_eq!(
            accepted_kinds(&column_type).is_some_and(|accepts| accepts.contains(&kind)),
            takes
        );
    }

    #[test]
    fn a_custom_type_is_not_checked() {
        assert_eq!(
            accepted_kinds(&ColumnType::Custom(sea_query::Alias::new("citext").into())),
            None
        );
    }
}
