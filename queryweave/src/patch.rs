//! The fields of a merge patch: what a client sent for each field of a model,
//! and how that applies to the model.
//!
//! A merge patch (JSON merge patch, RFC 7396) leaves out the fields it does
//! not change, gives a new value for the ones it replaces, and gives `null`
//! for the ones it clears. A field that cannot be cleared is an `Option<T>`
//! (absent or a value); one that can is a [`Patch<T>`] (absent, null or a
//! value).

/// A field of a merge patch that may also be cleared: absent from the patch,
/// set to `null`, or given a value.
///
/// With the `serde` feature, it deserializes `null` as [`Patch::Null`] and a
/// value as [`Patch::Value`]; an absent field takes the default,
/// [`Patch::Missing`], so put `#[serde(default)]` on the struct (or the
/// field). To document it with utoipa, give the field
/// `#[schema(value_type = Option<T>)]`: a nullable `T`, as clients send it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Patch<T> {
    /// Absent from the patch: the field is left unchanged.
    #[default]
    Missing,
    /// Set to `null`: the field is cleared.
    Null,
    /// Given a value: the field is replaced.
    Value(T),
}

impl<T> Patch<T> {
    /// Whether the field is absent from the patch.
    #[must_use]
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }

    /// Whether the field is set to `null`.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// The value given, if any (`None` when absent or `null`).
    #[must_use]
    pub fn value(self) -> Option<T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Missing | Self::Null => None,
        }
    }

    /// A reference to the value given, if any.
    #[must_use]
    pub const fn as_ref(&self) -> Patch<&T> {
        match self {
            Self::Missing => Patch::Missing,
            Self::Null => Patch::Null,
            Self::Value(value) => Patch::Value(value),
        }
    }

    /// The patch with its value, if any, mapped by `f`.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Patch<U> {
        match self {
            Self::Missing => Patch::Missing,
            Self::Null => Patch::Null,
            Self::Value(value) => Patch::Value(f(value)),
        }
    }

    /// The patch with its value, if any, mapped by the fallible `f` (e.g.
    /// parsing a value object), or `f`'s error.
    ///
    /// # Errors
    ///
    /// Returns `f`'s error.
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<Patch<U>, E> {
        Ok(match self {
            Self::Missing => Patch::Missing,
            Self::Null => Patch::Null,
            Self::Value(value) => Patch::Value(f(value)?),
        })
    }
}

/// A field of a merge patch, applied to the model's field of type `F`.
///
/// `Option<P>` replaces the field when it holds a value; `Patch<P>` also
/// clears an `Option` field when `null`. `P` converts into the field's type.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot patch a field of type `{F}`",
    label = "not a merge patch field for `{F}`",
    note = "a merge patch field is an `Option<P>`, or a `Patch<P>` for an `Option` field that may be cleared, with `P` converting into the field's type"
)]
pub trait PatchField<F> {
    /// Apply the patch's field to the model's `field`.
    fn apply_to(self, field: &mut F);
}

impl<P: Into<F>, F> PatchField<F> for Option<P> {
    fn apply_to(self, field: &mut F) {
        if let Some(value) = self {
            *field = value.into();
        }
    }
}

impl<P: Into<F>, F> PatchField<Option<F>> for Patch<P> {
    fn apply_to(self, field: &mut Option<F>) {
        match self {
            Self::Missing => {},
            Self::Null => *field = None,
            Self::Value(value) => *field = Some(value.into()),
        }
    }
}

/// What a merge patch field can say: whether its kind of field may be set to
/// `null` at all, and whether this one is.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a merge patch field",
    label = "neither an `Option<T>` nor a `Patch<T>`",
    note = "a merge patch field is an `Option<T>` (it cannot be cleared) or a `queryweave::Patch<T>` (it can); mark a field that is not patched (e.g. an id) `#[merge_patch(skip)]`"
)]
pub trait PatchKind {
    /// Whether a field of this kind may be set to `null`.
    const NULLABLE: bool;

    /// Whether this field is set to `null`.
    fn is_null(&self) -> bool;
}

impl<T> PatchKind for Option<T> {
    const NULLABLE: bool = false;

    fn is_null(&self) -> bool {
        false
    }
}

impl<T> PatchKind for Patch<T> {
    const NULLABLE: bool = true;

    fn is_null(&self) -> bool {
        Self::is_null(self)
    }
}

#[cfg(feature = "serde")]
mod serde_impls {
    use super::Patch;

    /// `null` is [`Patch::Null`], a value [`Patch::Value`]; an absent field is
    /// the default, [`Patch::Missing`], through `#[serde(default)]`.
    impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Patch<T> {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Ok(Option::<T>::deserialize(deserializer)?.map_or(Self::Null, Self::Value))
        }
    }

    /// A value as itself, `null` (and an absent field, which a struct skips
    /// with `#[serde(skip_serializing_if = "Patch::is_missing")]`) as `null`.
    impl<T: serde::Serialize> serde::Serialize for Patch<T> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                Self::Value(value) => serializer.serialize_some(value),
                Self::Missing | Self::Null => serializer.serialize_none(),
            }
        }
    }
}

#[cfg(feature = "schemars")]
mod schemars_impls {
    use super::Patch;

    /// Documented as a nullable `T`, as clients send it: an absent field is
    /// not in the request at all.
    impl<T: schemars::JsonSchema> schemars::JsonSchema for Patch<T> {
        fn inline_schema() -> bool {
            <Option<T> as schemars::JsonSchema>::inline_schema()
        }

        fn schema_name() -> std::borrow::Cow<'static, str> {
            <Option<T> as schemars::JsonSchema>::schema_name()
        }

        fn schema_id() -> std::borrow::Cow<'static, str> {
            <Option<T> as schemars::JsonSchema>::schema_id()
        }

        fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            <Option<T> as schemars::JsonSchema>::json_schema(generator)
        }
    }
}

#[cfg(feature = "validator")]
mod validator_impls {
    use super::Patch;

    /// validator's `range` rule checks a given value, and passes an absent
    /// field or a `null` (which `ValidateNonNullable` checks), as it does an
    /// `Option`.
    /// Delegates to the value's own check, so every type validator ranges
    /// over works.
    impl<T: validator::ValidateRange<T>> validator::ValidateRange<T> for Patch<T> {
        fn greater_than(&self, max: T) -> Option<bool> {
            match self {
                Self::Value(value) => value.greater_than(max),
                Self::Missing | Self::Null => None,
            }
        }

        fn less_than(&self, min: T) -> Option<bool> {
            match self {
                Self::Value(value) => value.less_than(min),
                Self::Missing | Self::Null => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Patch, PatchField, PatchKind};

    // A missing field leaves the model's field, `null` clears it, a value
    // replaces it.
    #[test]
    fn test_a_patch_applies_to_an_option_field() {
        let mut field = Some(1);

        Patch::<i32>::Missing.apply_to(&mut field);
        assert_eq!(field, Some(1));
        Patch::Value(2).apply_to(&mut field);
        assert_eq!(field, Some(2));
        Patch::<i32>::Null.apply_to(&mut field);
        assert_eq!(field, None);
    }

    // An `Option` replaces the field when it holds a value, and cannot clear it.
    #[test]
    fn test_an_option_applies_to_any_field() {
        let mut field = 1;

        None::<i32>.apply_to(&mut field);
        assert_eq!(field, 1);
        Some(2).apply_to(&mut field);
        assert_eq!(field, 2);
    }

    #[test]
    fn test_only_a_patch_may_be_null() {
        const { assert!(<Patch<i32> as PatchKind>::NULLABLE) };
        const { assert!(!<Option<i32> as PatchKind>::NULLABLE) };
        assert!(PatchKind::is_null(&Patch::<i32>::Null));
        assert!(!PatchKind::is_null(&Patch::Value(1)));
        assert!(!PatchKind::is_null(&None::<i32>));
    }

    // `Missing` is the default, whatever `T` is (no `T: Default` needed).
    #[test]
    fn test_the_default_is_missing() {
        struct NoDefault;

        assert!(Patch::<NoDefault>::default().is_missing());
    }

    #[test]
    fn test_map_and_try_map_keep_the_state() {
        assert_eq!(Patch::Value(2).map(|value| value * 2), Patch::Value(4));
        assert_eq!(Patch::<i32>::Null.map(|value| value * 2), Patch::Null);
        assert_eq!(
            Patch::Value("x").try_map(str::parse::<i32>).map_err(drop),
            Err(())
        );
        assert_eq!(
            Patch::<&str>::Missing.try_map(str::parse::<i32>),
            Ok(Patch::Missing)
        );
        assert_eq!(Patch::Value(3).value(), Some(3));
        assert_eq!(Patch::<i32>::Null.value(), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_reads_null_and_values_and_defaults_to_missing() {
        #[derive(Default, serde::Deserialize)]
        #[serde(default)]
        struct Body {
            a: Patch<i32>,
            b: Patch<i32>,
            c: Patch<i32>,
        }

        let body: Body = serde_json::from_value(serde_json::json!({ "a": null, "b": 1 })).unwrap();

        assert_eq!(body.a, Patch::Null);
        assert_eq!(body.b, Patch::Value(1));
        assert_eq!(body.c, Patch::Missing);
    }

    #[cfg(feature = "validator")]
    #[test]
    fn test_validator_range_checks_only_a_given_value() {
        use validator::ValidateRange;

        assert!(!Patch::Value(-1).validate_range(Some(0), None, None, None));
        assert!(Patch::Value(1).validate_range(Some(0), None, None, None));
        assert!(Patch::<i32>::Null.validate_range(Some(0), None, None, None));
        assert!(Patch::<i32>::Missing.validate_range(Some(0), None, None, None));
    }

    #[cfg(feature = "schemars")]
    #[test]
    fn test_schemars_documents_a_nullable_value() {
        assert_eq!(
            schemars::schema_for!(Patch<i32>),
            schemars::schema_for!(Option<i32>)
        );
    }
}
