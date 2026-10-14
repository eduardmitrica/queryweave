#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The `queryweave_sql::diesel` backend run against a real SQLite (in memory),
//! through Diesel's own boxed queries and changesets: the same cases as
//! `sqlite.rs`, the sqlx backend's.
//!
//! `Coupons` stands in for the code `Columns` generates for a Diesel entity:
//! a `match` from each domain field to its typed column.

use chrono::{DateTime, TimeZone, Utc};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use pretty_assertions::assert_eq;
use queryweave::{
    FieldValue, FilterCondition, FilterOperator, IntoValue, SortDirectionModel, SortKey, Value,
};
use queryweave_sql::Junction;
use queryweave_sql::diesel::{
    BoxedCondition, BoxedOrdering, ValueMismatch, ValueOf, assignment, condition, join, ordering,
    page,
};

/// The schema, as Diesel's CLI would print it.
mod schema {
    diesel::table! {
        coupons (id) {
            id -> Integer,
            label -> Nullable<Text>,
            quantity -> Integer,
            expires_at -> TimestamptzSqlite,
            is_active -> Bool,
        }
    }
}

use schema::coupons;

/// The `coupons` table's columns, by domain field: `amount` is stored in
/// `quantity`.
struct Coupons;

/// The changes a merge patch writes: one `Option` per column.
type Changeset = (
    Option<diesel::dsl::Eq<coupons::label, ValueOf<coupons::label>>>,
    Option<diesel::dsl::Eq<coupons::quantity, ValueOf<coupons::quantity>>>,
    Option<diesel::dsl::Eq<coupons::expires_at, ValueOf<coupons::expires_at>>>,
    Option<diesel::dsl::Eq<coupons::is_active, ValueOf<coupons::is_active>>>,
);

impl Coupons {
    fn condition(
        filter_condition: FilterCondition,
    ) -> Result<BoxedCondition<'static, coupons::table, Sqlite>, ValueMismatch> {
        match filter_condition.field {
            "label" => condition(coupons::label, filter_condition),
            "amount" => condition(coupons::quantity, filter_condition),
            "expires_at" => condition(coupons::expires_at, filter_condition),
            "is_active" => condition(coupons::is_active, filter_condition),
            field => Err(ValueMismatch { field }),
        }
    }

    fn ordering(sort_key: SortKey) -> Option<BoxedOrdering<'static, coupons::table, Sqlite>> {
        match sort_key.field {
            "label" => Some(ordering(coupons::label, sort_key)),
            "amount" => Some(ordering(coupons::quantity, sort_key)),
            "expires_at" => Some(ordering(coupons::expires_at, sort_key)),
            "is_active" => Some(ordering(coupons::is_active, sort_key)),
            _ => None,
        }
    }

    fn changeset(field_values: Vec<FieldValue>) -> Result<Changeset, ValueMismatch> {
        let mut changeset: Changeset = (None, None, None, None);
        for field_value in field_values {
            match field_value.field {
                "label" => changeset.0 = Some(assignment(coupons::label, field_value)?),
                "amount" => changeset.1 = Some(assignment(coupons::quantity, field_value)?),
                "expires_at" => changeset.2 = Some(assignment(coupons::expires_at, field_value)?),
                "is_active" => changeset.3 = Some(assignment(coupons::is_active, field_value)?),
                field => return Err(ValueMismatch { field }),
            }
        }
        Ok(changeset)
    }
}

fn expiry(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
}

/// A fresh in-memory database with three coupons.
fn coupons() -> SqliteConnection {
    let mut connection = SqliteConnection::establish(":memory:")
        .expect("Failed to open an in-memory SQLite database");

    diesel::sql_query(
        "CREATE TABLE coupons (
            id INTEGER PRIMARY KEY,
            label TEXT,
            quantity INTEGER NOT NULL,
            expires_at TEXT NOT NULL,
            is_active BOOLEAN NOT NULL
        )",
    )
    .execute(&mut connection)
    .unwrap();

    for (id, label, quantity, day, is_active) in [
        (1, "50%_off!", 5, 1, true),
        (2, "50 off", 7, 2, true),
        (3, "plain", 3, 3, false),
    ] {
        diesel::insert_into(coupons::table)
            .values((
                coupons::id.eq(id),
                coupons::label.eq(label),
                coupons::quantity.eq(quantity),
                coupons::expires_at.eq(expiry(day)),
                coupons::is_active.eq(is_active),
            ))
            .execute(&mut connection)
            .unwrap();
    }

    connection
}

/// The ids of the coupons matching `conditions`, sorted by `sort_keys` and
/// paged.
fn ids_where(
    connection: &mut SqliteConnection,
    conditions: Vec<FilterCondition>,
    sort_keys: Vec<SortKey>,
    page_request: (Option<u32>, Option<u32>),
) -> Vec<i32> {
    let mut query = coupons::table.select(coupons::id).into_boxed();

    let conditions = conditions
        .into_iter()
        .map(Coupons::condition)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if let Some(condition) = join(conditions, Junction::And) {
        query = query.filter(condition);
    }
    for sort_key in sort_keys {
        query = query.then_order_by(Coupons::ordering(sort_key).unwrap());
    }
    if let Some((limit, offset)) = page(page_request.0, page_request.1) {
        query = query.limit(limit).offset(offset);
    }

    query.load(connection).unwrap()
}

const fn filter(field: &'static str, operator: FilterOperator, value: Value) -> FilterCondition {
    FilterCondition {
        field,
        operator,
        value,
    }
}

fn by_amount() -> Vec<SortKey> {
    vec![SortKey {
        field: "amount",
        direction: SortDirectionModel::Ascending,
    }]
}

// `%`, `_` and the escape character `!` in the needle match literally: only
// the coupon whose label contains `%_off!` matches, not `50 off`.
#[test]
fn like_metacharacters_match_literally() {
    let mut connection = coupons();

    let ids = ids_where(
        &mut connection,
        vec![filter(
            "label",
            FilterOperator::Contains,
            "%_off!".to_owned().into_value(),
        )],
        Vec::new(),
        (None, None),
    );

    assert_eq!(ids, vec![1]);
}

// A widened `u32`, a date-time and a boolean compare against their columns.
#[test]
fn typed_values_compare_against_their_columns() {
    let mut connection = coupons();

    let ids = ids_where(
        &mut connection,
        vec![
            filter("amount", FilterOperator::GreaterThan, 4_u16.into_value()),
            filter(
                "expires_at",
                FilterOperator::LessThan,
                expiry(2).into_value(),
            ),
            filter("is_active", FilterOperator::Equal, true.into_value()),
        ],
        by_amount(),
        (None, None),
    );

    assert_eq!(ids, vec![1]);
}

// Sorting by a renamed field sorts by its column, and a page skips and takes
// rows.
#[test]
fn sorting_and_paging_apply() {
    let mut connection = coupons();
    let descending = vec![SortKey {
        field: "amount",
        direction: SortDirectionModel::Descending,
    }];

    assert_eq!(
        ids_where(
            &mut connection,
            Vec::new(),
            descending.clone(),
            (None, None)
        ),
        vec![2, 1, 3]
    );
    assert_eq!(
        ids_where(&mut connection, Vec::new(), descending, (Some(1), Some(2))),
        vec![3]
    );
}

// A changeset writes typed values (and `NULL`s) to the columns, through
// Diesel's own `update(..).set(..)`.
#[test]
fn assignments_write_typed_values() {
    let mut connection = coupons();

    let changeset = Coupons::changeset(vec![
        FieldValue {
            field: "label",
            value: Value::Text(None),
        },
        FieldValue {
            field: "amount",
            value: 42_i32.into_value(),
        },
        FieldValue {
            field: "expires_at",
            value: expiry(9).into_value(),
        },
    ])
    .unwrap();
    diesel::update(coupons::table.filter(coupons::id.eq(3)))
        .set(changeset)
        .execute(&mut connection)
        .unwrap();

    let row: (Option<String>, i32, DateTime<Utc>) = coupons::table
        .filter(coupons::id.eq(3))
        .select((coupons::label, coupons::quantity, coupons::expires_at))
        .first(&mut connection)
        .unwrap();
    assert_eq!(row, (None, 42, expiry(9)));
}

// A value that does not fit its column fails the condition, not the query.
#[test]
fn a_value_that_does_not_fit_is_reported() {
    let result = Coupons::condition(filter(
        "amount",
        FilterOperator::Equal,
        "five".to_owned().into_value(),
    ));

    assert_eq!(result.err(), Some(ValueMismatch { field: "amount" }));
}
