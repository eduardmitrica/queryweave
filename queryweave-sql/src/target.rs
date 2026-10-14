//! The queries a model's data is written into.
//!
//! [`Columns`]' methods check, at compile time, that every field of a model
//! has a column on the entity, then hand the data and the entity's column
//! mapping to the query being built: a *target*, implementing one trait per
//! clause it can take. The derives' generated code is generic over these
//! traits, so it names no query builder, and a target decides how the clause
//! is written (sqlx's [`QueryBuilder`](crate::QueryBuilder) appends SQL text
//! and binds the values).
//!
//! The `write_*` methods do not check the columns themselves: call them
//! through the entity's `Columns` methods (or the wrappers'
//! `append_to_query`), which do.

use crate::Columns;
#[cfg(feature = "merge-patch")]
use queryweave::FieldValues;
use queryweave::{Conditions, SortKeys};

/// A query that filter conditions can be written into: a `WHERE` clause.
pub trait ConditionsTarget {
    /// Start a `WHERE` clause, before its conditions.
    fn write_where(&mut self);

    /// Write the conditions of the model `M`, joined by `separator` (e.g.
    /// `AND`), each comparing the column `C` stores its domain field in.
    ///
    /// Values are bound with their own SQL type; the `LIKE` operators bind an
    /// escaped pattern (`… LIKE ? ESCAPE '!'`), so user input matches
    /// literally.
    fn write_conditions<C: Columns + ?Sized, M>(
        &mut self,
        conditions: Conditions<M>,
        separator: &str,
    );
}

/// A query that sort keys can be written into: an `ORDER BY` clause.
pub trait OrderByTarget {
    /// Write an `ORDER BY column direction, …` clause for the sort keys of the
    /// model `M`, in order, each by the column `C` stores its domain field
    /// in; nothing when there are none.
    fn write_order_by<C: Columns + ?Sized, M>(&mut self, sort_keys: SortKeys<M>);
}

/// A query that a page can be written into: `LIMIT` and `OFFSET`.
pub trait PageTarget {
    /// Write a `LIMIT page_size OFFSET page * page_size` clause (`page` 0
    /// based) when both a page and a page size are requested; nothing
    /// otherwise.
    fn write_page(&mut self, page: Option<u32>, page_size: Option<u32>);
}

/// A query that merge-patch writes can be written into: the assignments of an
/// `UPDATE … SET`.
#[cfg(feature = "merge-patch")]
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
