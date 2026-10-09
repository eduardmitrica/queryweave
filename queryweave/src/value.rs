//! Database-agnostic values the domain hands to the persistence layer.
//!
//! The derives describe *what* to filter on or write as plain data built from
//! these types; turning them into SQL (and binding them) is the persistence
//! layer's job, so the domain needs no database dependency.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

/// A raw value to compare against or store, in a database-agnostic form.
///
/// Every variant holds an `Option`, so an absent value (`NULL`) keeps its type.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A boolean.
    Bool(Option<bool>),
    /// A 16-bit integer.
    I16(Option<i16>),
    /// A 32-bit integer.
    I32(Option<i32>),
    /// A 64-bit integer.
    I64(Option<i64>),
    /// A 32-bit float.
    F32(Option<f32>),
    /// A 64-bit float.
    F64(Option<f64>),
    /// Text.
    Text(Option<String>),
    /// A UTC date-time.
    DateTimeUtc(Option<DateTime<Utc>>),
    /// A date-time without a time zone.
    NaiveDateTime(Option<NaiveDateTime>),
    /// A date.
    NaiveDate(Option<NaiveDate>),
    /// A time of day.
    NaiveTime(Option<NaiveTime>),
}

/// A raw value that can be handed to the persistence layer as a [`Value`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be filtered on or stored",
    label = "no `queryweave::Value` variant for this raw value",
    note = "supported raw values: bool, i8..i64, u8..u32, f32, f64, String and the chrono date/time types (and `Option`s of them); `u64` is not supported, since no Postgres integer holds its whole range: use `i64`, or add a `Value` variant (e.g. for `NUMERIC`)"
)]
pub trait IntoValue {
    /// Convert into a [`Value`].
    fn into_value(self) -> Value;
}

/// Map each raw type to its [`Value`] variant, for both `T` and `Option<T>`.
macro_rules! impl_into_value {
    ($($ty:ty => $variant:ident),* $(,)?) => {
        $(
            impl IntoValue for $ty {
                fn into_value(self) -> Value {
                    Value::$variant(Some(self))
                }
            }

            impl IntoValue for Option<$ty> {
                fn into_value(self) -> Value {
                    Value::$variant(self)
                }
            }
        )*
    };
}

impl_into_value!(
    bool => Bool,
    i16 => I16,
    i32 => I32,
    i64 => I64,
    f32 => F32,
    f64 => F64,
    String => Text,
    DateTime<Utc> => DateTimeUtc,
    NaiveDateTime => NaiveDateTime,
    NaiveDate => NaiveDate,
    NaiveTime => NaiveTime,
);

/// Map each integer type without a Postgres type of its own to the variant of
/// the smallest Postgres integer that holds its whole range (`i8`/`u8` to
/// `smallint`, `u16` to `integer`, `u32` to `bigint`), for both `T` and
/// `Option<T>`. The widening is lossless, and Postgres compares across integer
/// widths and assignment-casts on writes, so a narrower column still works
/// (an out-of-range write is a runtime "out of range" error, never a silent
/// truncation).
macro_rules! impl_into_value_widened {
    ($($ty:ty => $variant:ident($wide:ty)),* $(,)?) => {
        $(
            impl IntoValue for $ty {
                fn into_value(self) -> Value {
                    Value::$variant(Some(<$wide>::from(self)))
                }
            }

            impl IntoValue for Option<$ty> {
                fn into_value(self) -> Value {
                    Value::$variant(self.map(<$wide>::from))
                }
            }
        )*
    };
}

impl_into_value_widened!(
    i8 => I16(i16),
    u8 => I16(i16),
    u16 => I32(i32),
    u32 => I64(i64),
);

/// One set filter of a generated filter-options model: `field operator value`.
///
/// `field` is the domain model's field name; which column it is stored in is
/// the persistence layer's business.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterCondition {
    /// The domain field compared.
    pub field: &'static str,
    /// The comparison.
    pub operator: crate::FilterOperator,
    /// The value compared with.
    pub value: Value,
}

/// A domain field to write and the value to write to it, e.g. one changed
/// field of a merge patch.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldValue {
    /// The domain field written.
    pub field: &'static str,
    /// The value written.
    pub value: Value,
}

/// One sort key of a generated query model: sort by `field` in `direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortKey {
    /// The domain field sorted by.
    pub field: &'static str,
    /// The direction sorted in.
    pub direction: crate::SortDirectionModel,
}

#[cfg(test)]
mod tests {
    use super::{IntoValue, Value};

    #[test]
    fn a_raw_value_converts_into_its_present_variant() {
        assert_eq!(7_i32.into_value(), Value::I32(Some(7)));
        assert_eq!(
            String::from("x").into_value(),
            Value::Text(Some(String::from("x")))
        );
    }

    // An absent optional value keeps its type, so it can be bound as a typed
    // `NULL`.
    #[test]
    fn an_absent_optional_value_keeps_its_variant() {
        assert_eq!(None::<String>.into_value(), Value::Text(None));
        assert_eq!(Some(1.5_f64).into_value(), Value::F64(Some(1.5)));
    }

    // Integers without a Postgres type of their own widen losslessly to the
    // smallest Postgres integer holding their whole range, extremes included.
    #[test]
    fn narrow_and_unsigned_integers_widen_losslessly() {
        assert_eq!(i8::MIN.into_value(), Value::I16(Some(-128)));
        assert_eq!(u8::MAX.into_value(), Value::I16(Some(255)));
        assert_eq!(u16::MAX.into_value(), Value::I32(Some(65_535)));
        assert_eq!(u32::MAX.into_value(), Value::I64(Some(4_294_967_295)));
    }

    #[test]
    fn widened_optional_integers_keep_their_variant() {
        assert_eq!(Some(3_u32).into_value(), Value::I64(Some(3)));
        assert_eq!(None::<u32>.into_value(), Value::I64(None));
        assert_eq!(None::<u8>.into_value(), Value::I16(None));
    }
}
