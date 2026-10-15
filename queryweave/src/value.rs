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

impl Value {
    /// Which variant the value is.
    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::Bool(_) => ValueKind::Bool,
            Self::I16(_) => ValueKind::I16,
            Self::I32(_) => ValueKind::I32,
            Self::I64(_) => ValueKind::I64,
            Self::F32(_) => ValueKind::F32,
            Self::F64(_) => ValueKind::F64,
            Self::Text(_) => ValueKind::Text,
            Self::DateTimeUtc(_) => ValueKind::DateTimeUtc,
            Self::NaiveDateTime(_) => ValueKind::NaiveDateTime,
            Self::NaiveDate(_) => ValueKind::NaiveDate,
            Self::NaiveTime(_) => ValueKind::NaiveTime,
        }
    }
}

/// A [`Value`] variant without its value: what a raw type converts into,
/// known at compile time ([`IntoValue::TYPE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// [`Value::Bool`].
    Bool,
    /// [`Value::I16`].
    I16,
    /// [`Value::I32`].
    I32,
    /// [`Value::I64`].
    I64,
    /// [`Value::F32`].
    F32,
    /// [`Value::F64`].
    F64,
    /// [`Value::Text`].
    Text,
    /// [`Value::DateTimeUtc`].
    DateTimeUtc,
    /// [`Value::NaiveDateTime`].
    NaiveDateTime,
    /// [`Value::NaiveDate`].
    NaiveDate,
    /// [`Value::NaiveTime`].
    NaiveTime,
}

impl ValueKind {
    /// The Rust type the variant holds, for messages.
    #[must_use]
    pub const fn rust_type(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Text => "String",
            Self::DateTimeUtc => "DateTime<Utc>",
            Self::NaiveDateTime => "NaiveDateTime",
            Self::NaiveDate => "NaiveDate",
            Self::NaiveTime => "NaiveTime",
        }
    }
}

/// What a raw type is as a [`Value`], at compile time: its variant, and
/// whether it may be absent (`NULL`, an `Option`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueType {
    /// The variant it converts into.
    pub kind: ValueKind,
    /// Whether the value may be `NULL`: an `Option`.
    pub nullable: bool,
}

/// A raw value that can be handed to the persistence layer as a [`Value`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be filtered on or stored",
    label = "no `queryweave::Value` variant for this raw value",
    note = "supported raw values: bool, i8..i64, u8..u32, f32, f64, String and the chrono date/time types (and `Option`s of them); `u64` is not supported, since no Postgres integer holds its whole range: use `i64`, or add a `Value` variant (e.g. for `NUMERIC`)"
)]
pub trait IntoValue {
    /// The [`Value`] variant [`into_value`](Self::into_value) gives, and
    /// whether it may be `NULL`: what the persistence layer checks a column's
    /// type against at compile time.
    const TYPE: ValueType;

    /// Convert into a [`Value`].
    fn into_value(self) -> Value;
}

/// Map each raw type to its [`Value`] variant, for both `T` and `Option<T>`.
macro_rules! impl_into_value {
    ($($ty:ty => $variant:ident),* $(,)?) => {
        $(
            impl IntoValue for $ty {
                const TYPE: ValueType = ValueType { kind: ValueKind::$variant, nullable: false };

                fn into_value(self) -> Value {
                    Value::$variant(Some(self))
                }
            }

            impl IntoValue for Option<$ty> {
                const TYPE: ValueType = ValueType { kind: ValueKind::$variant, nullable: true };

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
                const TYPE: ValueType = ValueType { kind: ValueKind::$variant, nullable: false };

                fn into_value(self) -> Value {
                    Value::$variant(Some(<$wide>::from(self)))
                }
            }

            impl IntoValue for Option<$ty> {
                const TYPE: ValueType = ValueType { kind: ValueKind::$variant, nullable: true };

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
    use super::{IntoValue, Value, ValueKind, ValueType};

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
    // The compile-time type is the variant `into_value` gives, and an
    // `Option` is the nullable one.
    #[test]
    fn the_type_is_the_variant_into_value_gives() {
        fn assert_type<T: IntoValue>(value: T, nullable: bool) {
            assert_eq!(
                T::TYPE,
                ValueType {
                    kind: value.into_value().kind(),
                    nullable
                }
            );
        }

        assert_type(true, false);
        assert_type(1_i16, false);
        assert_type(1_i32, false);
        assert_type(1_i64, false);
        assert_type(1.0_f32, false);
        assert_type(1.0_f64, false);
        assert_type(String::new(), false);
        assert_type(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH, false);
        assert_type(chrono::NaiveDateTime::default(), false);
        assert_type(chrono::NaiveDate::default(), false);
        assert_type(chrono::NaiveTime::default(), false);
        assert_type(1_i8, false);
        assert_type(1_u8, false);
        assert_type(1_u16, false);
        assert_type(1_u32, false);
        assert_type(None::<String>, true);
        assert_type(Some(1_u32), true);
    }

    #[test]
    fn a_kind_names_its_rust_type() {
        assert_eq!(ValueKind::Text.rust_type(), "String");
        assert_eq!(ValueKind::DateTimeUtc.rust_type(), "DateTime<Utc>");
    }
}
