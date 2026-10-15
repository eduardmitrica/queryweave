//! The compile-time check that each domain field's value fits its Diesel
//! column: the domain field's value type (`PersistedFields::FIELD_TYPES`)
//! against what its column takes (`FilterColumn::ACCEPTS` to filter on it,
//! `AssignColumn::TYPE` to write it).

use super::ColumnType;
use crate::Columns;
use crate::columns::{Message, column_for, str_eq};
use queryweave::{PersistedFields, ValueKind, ValueType};

/// Fail const evaluation unless the value of every field of the model `M`
/// fits its column on the Diesel entity `E`, for filtering on it.
///
/// `columns` is `(domain field, the variants its column takes)` for every
/// column of the entity, generated with its `diesel_conditions`, which calls
/// this in an inline
/// `const` block: evaluated once per instantiation, so it is reported by
/// `cargo build`, not by `cargo check`. A model without value types (a
/// hand-written `PersistedFields`) is not checked.
///
/// # Panics
///
/// When a field's value does not fit its column; called in a `const` block,
/// that panic is a compile error naming the field, the column and the entity.
// fails the build, not a test; covered by the trybuild UI cases (excluded from mutation runs)
#[mutants::skip]
pub const fn assert_condition_types<E: Columns + ?Sized, M: PersistedFields>(
    columns: &[(&str, &[ValueKind])],
) {
    assert_value_types::<E, M>(Target::Filter(columns));
}

/// Fail const evaluation unless the value of every field of the `MergePatch`
/// model `M` fits its column on the Diesel entity `E`, for writing it.
///
/// As for [`assert_condition_types`], and a field that may be `None` needs a
/// nullable column.
///
/// `columns` is `(domain field, column type)` for every column, generated
/// with the entity's `diesel_changeset`.
///
/// # Panics
///
/// When a field's value does not fit its column; called in a `const` block,
/// that panic is a compile error naming the field, the column and the entity.
// fails the build, not a test; covered by the trybuild UI cases (excluded from mutation runs)
#[mutants::skip]
pub const fn assert_assignment_types<E: Columns + ?Sized, M: PersistedFields>(
    columns: &[(&str, ColumnType)],
) {
    assert_value_types::<E, M>(Target::Write(columns));
}

#[allow(
    clippy::panic,
    reason = "a const-evaluation panic is how a compile-time check fails the build"
)]
const fn assert_value_types<E: Columns + ?Sized, M: PersistedFields>(target: Target<'_>) {
    if let Some(message) =
        value_type_mismatch(E::NAME, E::COLUMNS, target, M::FIELD_NAMES, M::FIELD_TYPES)
    {
        panic!("{}", message.as_str());
    }
}

/// The entity's columns, as the check needs them for what the data does.
#[derive(Clone, Copy)]
enum Target<'c> {
    /// Filtering: the variants each column takes (any column compares with
    /// `NULL`).
    Filter(&'c [(&'c str, &'c [ValueKind])]),
    /// Writing: each column's type, nullability included.
    Write(&'c [(&'c str, ColumnType)]),
}

/// The message for the first field of a model whose value does not fit its
/// column, if any; `None` too without value types (`types` not as long as
/// `fields`).
const fn value_type_mismatch(
    entity: &str,
    names: &'static [(&'static str, &'static str)],
    target: Target<'_>,
    fields: &[&str],
    types: &[ValueType],
) -> Option<Message> {
    if types.len() != fields.len() {
        return None;
    }

    let mut field = 0;
    while field < fields.len() {
        let (name, value_type) = (fields[field], types[field]);
        if let Some((accepted, nullable)) = column_type(target, name) {
            let column = match column_for(names, name) {
                Some(column) => column,
                None => name,
            };
            if !accepts(accepted, value_type.kind) {
                return Some(kind_message(
                    entity,
                    name,
                    column,
                    value_type.kind,
                    accepted,
                ));
            }
            if value_type.nullable && !nullable {
                return Some(nullable_message(entity, name, column));
            }
        }
        field += 1;
    }

    None
}

/// What the column storing `field` takes, if the entity has one: its
/// variants, and whether a `NULL` fits (always, to filter on it).
const fn column_type<'c>(target: Target<'c>, field: &str) -> Option<(&'c [ValueKind], bool)> {
    let mut index = 0;
    match target {
        Target::Filter(columns) => {
            while index < columns.len() {
                let (domain_field, accepts) = columns[index];
                if str_eq(domain_field, field) {
                    return Some((accepts, true));
                }
                index += 1;
            }
        },
        Target::Write(columns) => {
            while index < columns.len() {
                let (domain_field, column_type) = columns[index];
                if str_eq(domain_field, field) {
                    return Some((column_type.accepts, column_type.nullable));
                }
                index += 1;
            }
        },
    }

    None
}

/// Whether `kind` is one of `accepts`.
const fn accepts(accepts: &[ValueKind], kind: ValueKind) -> bool {
    let mut index = 0;
    while index < accepts.len() {
        if accepts[index] as u8 == kind as u8 {
            return true;
        }
        index += 1;
    }

    false
}

/// A field whose value is of a kind its column does not take.
const fn kind_message(
    entity: &str,
    field: &str,
    column: &str,
    kind: ValueKind,
    accepts: &[ValueKind],
) -> Message {
    let mut message = Message::new()
        .push("the value of domain field `")
        .push(field)
        .push("` (`")
        .push(kind.rust_type())
        .push("`) does not fit the column `")
        .push(column)
        .push("` of `")
        .push(entity)
        .push("`, which takes ");
    let mut index = 0;
    while index < accepts.len() {
        if index > 0 {
            message = message.push(" or ");
        }
        message = message.push("`").push(accepts[index].rust_type()).push("`");
        index += 1;
    }

    message.push(": make the field's type and the column's SQL type in `table!` agree")
}

/// A field that may be `None`, written to a `NOT NULL` column.
const fn nullable_message(entity: &str, field: &str, column: &str) -> Message {
    Message::new()
        .push("domain field `")
        .push(field)
        .push("` may be `None`, but the column `")
        .push(column)
        .push("` of `")
        .push(entity)
        .push("` is `NOT NULL`: make the column `Nullable` in `table!`, or the field not optional")
}

#[cfg(test)]
mod tests {
    use super::{ColumnType, Target, value_type_mismatch};
    use pretty_assertions::assert_eq;
    use queryweave::{ValueKind, ValueType};

    const NAMES: &[(&str, &str)] = &[("quantity", "quantity"), ("delivery_date", "delivered_at")];
    const COLUMNS: &[(&str, ColumnType)] = &[
        (
            "quantity",
            ColumnType {
                accepts: &[ValueKind::I32, ValueKind::I16],
                nullable: false,
            },
        ),
        (
            "delivery_date",
            ColumnType {
                accepts: &[ValueKind::DateTimeUtc],
                nullable: false,
            },
        ),
    ];
    const FILTER_COLUMNS: &[(&str, &[ValueKind])] = &[
        ("quantity", &[ValueKind::I32, ValueKind::I16]),
        ("delivery_date", &[ValueKind::DateTimeUtc]),
    ];
    const FIELDS: &[&str] = &["quantity", "delivery_date"];

    const fn value(kind: ValueKind, nullable: bool) -> ValueType {
        ValueType { kind, nullable }
    }

    fn mismatch(types: &[ValueType], writes: bool) -> Option<String> {
        let target = if writes {
            Target::Write(COLUMNS)
        } else {
            Target::Filter(FILTER_COLUMNS)
        };
        value_type_mismatch("Order", NAMES, target, FIELDS, types)
            .map(|message| message.as_str().to_owned())
    }

    #[test]
    fn values_of_a_kind_the_column_takes_fit() {
        let types = [
            value(ValueKind::I16, false),
            value(ValueKind::DateTimeUtc, false),
        ];

        assert_eq!(mismatch(&types, false), None);
        assert_eq!(mismatch(&types, true), None);
    }

    #[test]
    fn a_value_of_another_kind_names_the_field_column_and_kinds() {
        let types = [
            value(ValueKind::Text, false),
            value(ValueKind::DateTimeUtc, false),
        ];

        assert_eq!(
            mismatch(&types, false).as_deref(),
            Some(
                "the value of domain field `quantity` (`String`) does not fit the column `quantity` of `Order`, which takes `i32` or `i16`: make the field's type and the column's SQL type in `table!` agree"
            )
        );
    }

    // Filtering on a `NOT NULL` column with an optional field is fine; writing
    // `None` to it is not.
    #[test]
    fn a_nullable_value_needs_a_nullable_column_only_to_be_written() {
        let types = [
            value(ValueKind::I16, false),
            value(ValueKind::DateTimeUtc, true),
        ];

        assert_eq!(mismatch(&types, false), None);
        assert_eq!(
            mismatch(&types, true).as_deref(),
            Some(
                "domain field `delivery_date` may be `None`, but the column `delivered_at` of `Order` is `NOT NULL`: make the column `Nullable` in `table!`, or the field not optional"
            )
        );
    }

    // A hand-written `PersistedFields` has no value types: nothing checked.
    #[test]
    fn a_model_without_value_types_is_not_checked() {
        assert_eq!(mismatch(&[], true), None);
    }
}
