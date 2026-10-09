//! The raw (database-level) value behind a domain field, resolved by the type
//! system rather than guessed from the field type's name by the derives.

/// A domain field type with a raw value behind it: the value filters compare
/// with and merge patches bind.
///
/// Scalars, `String` and the `chrono` date-time types are their own raw value.
/// A value object gets this from `#[derive(RawValue)]`, with the raw value of
/// the type it wraps; `Option<T>` has the optional raw value of `T`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no raw value to filter on or bind",
    label = "not a scalar, `String`, date-time or value object",
    note = "derive `queryweave_derive::RawValue` on a value object, or implement `queryweave::RawValue`"
)]
pub trait RawValue {
    /// The raw value type.
    type Raw;

    /// A copy of the raw value.
    fn to_raw(&self) -> Self::Raw;
}

/// Types that are their own raw value.
macro_rules! impl_raw_value_for_self {
    ($($ty:ty),* $(,)?) => {
        $(
            impl RawValue for $ty {
                type Raw = Self;

                fn to_raw(&self) -> Self::Raw {
                    self.clone()
                }
            }
        )*
    };
}

impl_raw_value_for_self!(
    bool,
    i8,
    i16,
    i32,
    i64,
    u8,
    u16,
    u32,
    u64,
    f32,
    f64,
    String,
    chrono::DateTime<chrono::Utc>,
    chrono::NaiveDateTime,
    chrono::NaiveDate,
    chrono::NaiveTime,
);

impl<T: RawValue> RawValue for Option<T> {
    type Raw = Option<T::Raw>;

    fn to_raw(&self) -> Self::Raw {
        self.as_ref().map(RawValue::to_raw)
    }
}

/// A raw value the `lt` / `gt` filters can compare.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not support the `lt` / `gt` filters",
    label = "not ordered",
    note = "`lt` / `gt` apply to numbers and date-times; implement `queryweave::Ordered` to allow them"
)]
pub trait Ordered {}

/// A raw value the `sw` / `c` / `ew` (`LIKE`) filters can match.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not support the `sw` / `c` / `ew` filters",
    label = "not text",
    note = "`sw` / `c` / `ew` apply to `String`s"
)]
pub trait Text {}

/// A raw value the `bool` filter applies to.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not support the `bool` filter",
    label = "not a `bool`",
    note = "`bool` applies to `bool`s; use `eq` for other types"
)]
pub trait Boolean {}

/// Tag each type with a filter kind.
macro_rules! impl_marker {
    ($marker:ident: $($ty:ty),* $(,)?) => {
        $(impl $marker for $ty {})*
    };
}

impl_marker!(
    Ordered: i8,
    i16,
    i32,
    i64,
    u8,
    u16,
    u32,
    u64,
    f32,
    f64,
    chrono::DateTime<chrono::Utc>,
    chrono::NaiveDateTime,
    chrono::NaiveDate,
    chrono::NaiveTime,
);
impl_marker!(Text: String);
impl_marker!(Boolean: bool);

#[cfg(test)]
mod tests {
    use super::RawValue;

    #[derive(Debug, Clone, PartialEq)]
    struct Code(String);

    impl RawValue for Code {
        type Raw = String;

        fn to_raw(&self) -> Self::Raw {
            self.0.clone()
        }
    }

    #[test]
    fn a_scalar_is_its_own_raw_value() {
        assert_eq!(42_i32.to_raw(), 42);
        assert_eq!(String::from("x").to_raw(), "x");
    }

    #[test]
    fn an_option_maps_its_raw_value() {
        assert_eq!(
            Some(Code(String::from("x"))).to_raw(),
            Some(String::from("x"))
        );
        assert_eq!(None::<Code>.to_raw(), None);
    }
}
