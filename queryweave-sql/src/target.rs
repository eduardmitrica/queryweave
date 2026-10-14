//! The queries a model's data is written into.
//!
//! [`Columns`]' methods check, at compile time, that every field of a model
//! has a column on the entity, then hand the data and the entity's column
//! mapping to the query being built: a *target*, implementing one trait per
//! clause it can take. The derives' generated code is generic over these
//! traits, so it names no query builder, and a target decides how the clause
//! is written (sqlx's `QueryBuilder`, with the `sqlx` feature, appends SQL
//! text and binds the values; the public helpers of this crate, such as
//! `like_pattern` and `page_offset`, write it for a target of your own).
//!
//! The `write_*` methods do not check the columns themselves: call them
//! through the entity's `Columns` methods (or the wrappers'
//! `append_to_query`), which do.

use crate::Columns;
#[cfg(feature = "merge-patch")]
use queryweave::FieldValues;
use queryweave::{Conditions, SortKeys};

/// How conditions are joined: every one must hold (`AND`), or any (`OR`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Junction {
    /// Every condition must hold: `… AND …`.
    And,
    /// Any condition may hold: `… OR …`.
    Or,
}

/// A query that filter conditions can be written into: a `WHERE` clause.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a query `queryweave-sql` can write conditions into",
    label = "not a `ConditionsTarget`",
    note = "sqlx's `QueryBuilder` is one when `queryweave-sql`'s `sqlx` feature is enabled (it is by default): `queryweave-sql = {{ version = \"..\", features = [\"sqlx\"] }}`"
)]
pub trait ConditionsTarget {
    /// Start a `WHERE` clause, before its conditions.
    fn write_where(&mut self);

    /// Write the conditions of the model `M`, joined by `junction`, each
    /// comparing the column `C` stores its domain field in.
    ///
    /// Values are bound with their own SQL type; the `LIKE` operators bind an
    /// escaped pattern (`… LIKE ? ESCAPE '!'`), so user input matches
    /// literally.
    fn write_conditions<C: Columns + ?Sized, M>(
        &mut self,
        conditions: Conditions<M>,
        junction: Junction,
    );
}

/// A query that sort keys can be written into: an `ORDER BY` clause.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a query `queryweave-sql` can write an `ORDER BY` into",
    label = "not an `OrderByTarget`",
    note = "sqlx's `QueryBuilder` is one when `queryweave-sql`'s `sqlx` feature is enabled (it is by default): `queryweave-sql = {{ version = \"..\", features = [\"sqlx\"] }}`"
)]
pub trait OrderByTarget {
    /// Write an `ORDER BY column direction, …` clause for the sort keys of the
    /// model `M`, in order, each by the column `C` stores its domain field
    /// in; nothing when there are none.
    fn write_order_by<C: Columns + ?Sized, M>(&mut self, sort_keys: SortKeys<M>);
}

/// A query that a page can be written into: `LIMIT` and `OFFSET`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a query `queryweave-sql` can write a page into",
    label = "not a `PageTarget`",
    note = "sqlx's `QueryBuilder` is one when `queryweave-sql`'s `sqlx` feature is enabled (it is by default): `queryweave-sql = {{ version = \"..\", features = [\"sqlx\"] }}`"
)]
pub trait PageTarget {
    /// Write a `LIMIT page_size OFFSET page * page_size` clause (`page` 0
    /// based) when both a page and a page size are requested; nothing
    /// otherwise.
    fn write_page(&mut self, page: Option<u32>, page_size: Option<u32>);
}

/// A query that merge-patch writes can be written into: the assignments of an
/// `UPDATE … SET`.
#[cfg(feature = "merge-patch")]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a query `queryweave-sql` can write assignments into",
    label = "not an `AssignmentsTarget`",
    note = "sqlx's `QueryBuilder` is one when `queryweave-sql`'s `sqlx` feature is enabled (it is by default): `queryweave-sql = {{ version = \"..\", features = [\"sqlx\"] }}`"
)]
pub trait AssignmentsTarget {
    /// What writing the assignments returns, to add more of them (for sqlx,
    /// the comma-separated builder).
    type Assignments<'target>
    where
        Self: 'target;

    /// Write `column = value` assignments of the model `M`, each to the column
    /// `C` stores its domain field in.
    fn write_assignments<C: Columns + ?Sized, M>(
        &mut self,
        assignments: FieldValues<M>,
    ) -> Self::Assignments<'_>;
}
