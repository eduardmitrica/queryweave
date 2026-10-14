//! Mapping domain field names onto the columns they are stored in.
//!
//! The domain derives describe filters, sorting and writes in terms of domain
//! field names, and publish which fields reach the database
//! (`queryweave::PersistedFields`). An entity deriving `Columns`
//! (`queryweave-derive`) says which column each domain field lives in.
//! Every SQL helper of this crate checks, when it is instantiated for an
//! entity and a model, that each of the model's fields has a column, and
//! [`assert_columns!`](crate::assert_columns) does the same as a standalone
//! item, so a missing column is a build error rather than a runtime SQL error.

#[cfg(feature = "merge-patch")]
use crate::AssignmentsTarget;
use crate::{ConditionsTarget, Junction, OrderByTarget};
#[cfg(feature = "merge-patch")]
use queryweave::FieldValues;
use queryweave::{Conditions, PersistedFields, SortKeys};

/// A table (usually a `sqlx` entity) whose columns store a domain model's
/// fields, and the SQL written for that model's data against those columns.
///
/// The SQL methods are generic over the query they write into (a target, such
/// as sqlx's `QueryBuilder` for any `BindValues` database: Postgres,
/// MySQL/MariaDB, SQLite) and over the model `M`, both
/// taken from the arguments: `Order::push_assignments(&mut query_builder,
/// order.changed_values(&diff))`. Each fails to build unless every field of
/// `M` has a column on the table.
pub trait Columns {
    /// The table's type name, for compile errors about it.
    const NAME: &'static str;

    /// `(domain field, column)` for every column of the table.
    const COLUMNS: &'static [(&'static str, &'static str)];

    /// The column a domain field is stored in, if the table has one.
    #[must_use]
    fn column(field: &str) -> Option<&'static str> {
        column_for(Self::COLUMNS, field)
    }

    /// Push the conditions of the model `M` to a query, joined by `junction`,
    /// each a `column operator value` comparison against the column the table
    /// stores its domain field in.
    ///
    /// The conditions are a generated filter-options model's `conditions()`.
    /// Values are bound with their own SQL type; the `LIKE` operators bind an
    /// escaped pattern (`… LIKE ? ESCAPE '!'`), so user input matches
    /// literally.
    ///
    /// Fails to build unless every field of `M` has a column on the table.
    fn push_conditions<Q: ConditionsTarget, M: PersistedFields>(
        query: &mut Q,
        conditions: Conditions<M>,
        junction: Junction,
    ) {
        const { assert_model_columns::<Self, M>() };

        query.write_conditions::<Self, M>(conditions, junction);
    }

    /// Push an `ORDER BY column direction, …` clause for the sort keys of the
    /// model `M`; nothing when there are none.
    ///
    /// The sort keys are a generated query model's `sort_keys()`, written in
    /// order, each by the column the table stores its domain field in.
    ///
    /// Fails to build unless every field of `M` has a column on the table.
    fn push_order_by<Q: OrderByTarget, M: PersistedFields>(query: &mut Q, sort_keys: SortKeys<M>) {
        const { assert_model_columns::<Self, M>() };

        query.write_order_by::<Self, M>(sort_keys);
    }

    #[cfg(feature = "merge-patch")]
    /// Push `column = value` assignments of the model `M` for an
    /// `UPDATE … SET`, comma separated, returning what the query gives to add
    /// more (for sqlx, the separated builder).
    ///
    /// The assignments are a `MergePatch` model's `changed_values`, each to
    /// the column the table stores its domain field in.
    ///
    /// Fails to build unless every field of `M` has a column on the table.
    fn push_assignments<Q: AssignmentsTarget, M: PersistedFields>(
        query: &mut Q,
        assignments: FieldValues<M>,
    ) -> Q::Assignments<'_> {
        const { assert_model_columns::<Self, M>() };

        query.write_assignments::<Self, M>(assignments)
    }
}

/// Byte-wise string equality, usable in `const` code.
const fn str_eq(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }

    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }

    true
}

/// The column `field` is stored in, per a `(domain field, column)` table.
#[must_use]
pub const fn column_for(
    columns: &'static [(&'static str, &'static str)],
    field: &str,
) -> Option<&'static str> {
    let mut index = 0;
    while index < columns.len() {
        let (domain_field, column) = columns[index];
        if str_eq(domain_field, field) {
            return Some(column);
        }
        index += 1;
    }

    None
}

/// The first of `fields` without a column in the `(domain field, column)`
/// table, if any. Usable in `const` code, which is how
/// [`assert_columns!`](crate::assert_columns) checks coverage at compile time.
#[must_use]
pub const fn first_field_without_column(
    columns: &'static [(&'static str, &'static str)],
    fields: &'static [&'static str],
) -> Option<&'static str> {
    let mut index = 0;
    while index < fields.len() {
        if column_for(columns, fields[index]).is_none() {
            return Some(fields[index]);
        }
        index += 1;
    }

    None
}

/// The largest message [`missing_column_message`] builds; longer ones are cut.
const MESSAGE_CAPACITY: usize = 512;

/// A message built at compile time (see [`missing_column_message`]).
pub struct Message {
    bytes: [u8; MESSAGE_CAPACITY],
    len: usize,
}

impl Message {
    /// Append `text`, cutting it at the capacity.
    const fn push(mut self, text: &str) -> Self {
        let text = text.as_bytes();
        let mut index = 0;
        while index < text.len() && self.len < MESSAGE_CAPACITY {
            self.bytes[self.len] = text[index];
            self.len += 1;
            index += 1;
        }

        self
    }

    /// The message text.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match core::str::from_utf8(self.bytes.split_at(self.len).0) {
            Ok(text) => text,
            // Only a cut through a multi-byte character gets here.
            Err(_) => "a domain field has no column",
        }
    }
}

/// The compile error [`assert_columns!`](crate::assert_columns) reports for a
/// domain field without a column on `entity`. Built in `const` code, since a
/// `const` panic cannot format its message.
#[must_use]
pub const fn missing_column_message(entity: &str, field: &str) -> Message {
    Message {
        bytes: [0; MESSAGE_CAPACITY],
        len: 0,
    }
    .push("domain field `")
    .push(field)
    .push("` has no column on `")
    .push(entity)
    .push("`: add a field of that name to the entity, or map one onto it with `#[domain_field(")
    .push(field)
    .push(")]`")
}

/// Assert at compile time that every one of `fields` has a column on `entity`.
///
/// `fields` is a `&'static [&'static str]`, e.g. a model's
/// `<Model as queryweave::PersistedFields>::FIELD_NAMES`; `entity` a type
/// implementing [`Columns`].
///
/// Expands to an anonymous `const` item, so a missing column fails the build,
/// `cargo check` included, with a message naming the field and the entity. The
/// SQL helpers of this crate run the same check themselves when instantiated
/// for an entity and a model; this standalone form additionally reports it in
/// `cargo check` and the editor.
///
/// # Examples
///
/// ```rust
/// struct Order;
///
/// impl queryweave_sql::Columns for Order {
///     const NAME: &'static str = "Order";
///     const COLUMNS: &'static [(&'static str, &'static str)] =
///         &[("number", "number"), ("delivery_date", "delivered_at")];
/// }
///
/// queryweave_sql::assert_columns!(Order, &["number", "delivery_date"]);
/// ```
#[macro_export]
macro_rules! assert_columns {
    ($entity:ty, $fields:expr $(,)?) => {
        const _: () = {
            if let ::core::option::Option::Some(field) =
                $crate::first_field_without_column(<$entity as $crate::Columns>::COLUMNS, $fields)
            {
                ::core::panic!(
                    "{}",
                    $crate::missing_column_message(<$entity as $crate::Columns>::NAME, field)
                        .as_str()
                );
            }
        };
    };
}

/// Fail const evaluation unless every field of the model `M` has a column on
/// the entity `C`.
///
/// The SQL helpers call it in an inline `const` block, evaluated once per
/// instantiation, so writing SQL for a model through an entity lacking a
/// column for one of its fields fails to build. Being evaluated on
/// instantiation, it is reported by `cargo build` / `cargo test`, not by
/// `cargo check`.
///
/// # Panics
///
/// When a field of `M` has no column on `C`; called in a `const` block, that
/// panic is a compile error naming the field and the entity.
#[allow(
    clippy::panic,
    reason = "a const-evaluation panic is how a compile-time check fails the build"
)]
// fails the build, not a test; covered by the trybuild UI cases (excluded from mutation runs)
#[mutants::skip]
pub const fn assert_model_columns<C: Columns + ?Sized, M: PersistedFields>() {
    if let Some(field) = first_field_without_column(C::COLUMNS, M::FIELD_NAMES) {
        panic!("{}", missing_column_message(C::NAME, field).as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::{Columns, column_for, first_field_without_column, missing_column_message, str_eq};
    use pretty_assertions::assert_eq;

    struct Order;

    impl Columns for Order {
        const NAME: &'static str = "Order";
        const COLUMNS: &'static [(&'static str, &'static str)] =
            &[("number", "number"), ("delivery_date", "delivered_at")];
    }

    #[test]
    fn str_eq_compares_whole_strings() {
        assert!(str_eq("number", "number"));
        assert!(!str_eq("number", "numbers"));
        assert!(!str_eq("number", "Number"));
        assert!(str_eq("", ""));
    }

    #[test]
    fn a_field_maps_onto_its_column() {
        assert_eq!(Order::column("number"), Some("number"));
        assert_eq!(Order::column("delivery_date"), Some("delivered_at"));
        assert_eq!(Order::column("client"), None);
        assert_eq!(column_for(Order::COLUMNS, "delivered_at"), None);
    }

    #[test]
    fn the_first_field_without_a_column_is_reported() {
        assert_eq!(
            first_field_without_column(Order::COLUMNS, &["number", "delivery_date"]),
            None
        );
        assert_eq!(
            first_field_without_column(Order::COLUMNS, &["number", "client", "value"]),
            Some("client")
        );
    }

    // The check runs in `const` code; this compiling is the passing case.
    crate::assert_columns!(Order, &["delivery_date", "number"]);

    #[test]
    fn the_message_names_the_field_and_the_entity() {
        assert_eq!(
            missing_column_message("Order", "client").as_str(),
            "domain field `client` has no column on `Order`: add a field of that name to the entity, or map one onto it with `#[domain_field(client)]`"
        );
    }

    #[test]
    fn an_overlong_message_is_cut_at_the_capacity() {
        let field = "f".repeat(600);

        assert_eq!(
            missing_column_message("Order", &field).as_str().len(),
            super::MESSAGE_CAPACITY
        );
    }
}
