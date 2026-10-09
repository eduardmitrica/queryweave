#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The SQL written by `queryweave_sql` run against a real SQLite (in
//! memory), the database most unlike Postgres: `?` placeholders, no default
//! `LIKE` escape character, dynamic typing.

use chrono::{DateTime, TimeZone, Utc};
use pretty_assertions::assert_eq;
use queryweave::{
    FieldValue, FilterCondition, FilterOperator, IntoValue, PersistedFields, SortDirectionModel,
    SortKey, Value,
};
use queryweave_sql::{Columns, push_page};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

/// The domain model the tests write: a coupon, its amount stored in the
/// `quantity` column.
struct Coupon;

impl PersistedFields for Coupon {
    const FIELD_NAMES: &'static [&'static str] = &["label", "amount", "expires_at", "is_active"];
}

/// The `coupons` table.
struct Coupons;

impl Columns for Coupons {
    const NAME: &'static str = "Coupons";
    const COLUMNS: &'static [(&'static str, &'static str)] = &[
        ("label", "label"),
        ("amount", "quantity"),
        ("expires_at", "expires_at"),
        ("is_active", "is_active"),
    ];
}

fn expiry(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
}

/// A fresh in-memory database (one connection, since each in-memory
/// connection is its own database) with three coupons.
async fn coupons() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to open an in-memory SQLite database");

    sqlx::query(
        "CREATE TABLE coupons (
            id INTEGER PRIMARY KEY,
            label TEXT,
            quantity INTEGER NOT NULL,
            expires_at TEXT NOT NULL,
            is_active BOOLEAN NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    for (id, label, quantity, day, is_active) in [
        (1, "50%_off!", 5, 1, true),
        (2, "50 off", 7, 2, true),
        (3, "plain", 3, 3, false),
    ] {
        sqlx::query("INSERT INTO coupons VALUES (?, ?, ?, ?, ?)")
            .bind(id)
            .bind(label)
            .bind(quantity)
            .bind(expiry(day))
            .bind(is_active)
            .execute(&pool)
            .await
            .unwrap();
    }

    pool
}

/// The ids of the coupons matching `conditions`, sorted by `sort_keys` and
/// paged.
async fn ids_where(
    pool: &SqlitePool,
    conditions: Vec<FilterCondition>,
    sort_keys: Vec<SortKey>,
    page: (Option<u32>, Option<u32>),
) -> Vec<i64> {
    let mut query_builder = QueryBuilder::<Sqlite>::new("SELECT id FROM coupons ");
    if !conditions.is_empty() {
        query_builder.push("WHERE ");
        Coupons::push_conditions(
            &mut query_builder,
            queryweave::__private::model_data::<Coupon, _>(conditions),
            "AND",
        );
    }
    Coupons::push_order_by(
        &mut query_builder,
        queryweave::__private::model_data::<Coupon, _>(sort_keys),
    );
    push_page(&mut query_builder, page.0, page.1);

    query_builder
        .build_query_scalar::<i64>()
        .fetch_all(pool)
        .await
        .unwrap()
}

const fn condition(field: &'static str, operator: FilterOperator, value: Value) -> FilterCondition {
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
#[tokio::test]
async fn like_metacharacters_match_literally() {
    let pool = coupons().await;

    let ids = ids_where(
        &pool,
        vec![condition(
            "label",
            FilterOperator::Contains,
            "%_off!".to_owned().into_value(),
        )],
        Vec::new(),
        (None, None),
    )
    .await;

    assert_eq!(ids, vec![1]);
}

// A widened `u32`, a date-time and a boolean compare against their columns.
#[tokio::test]
async fn typed_values_compare_against_their_columns() {
    let pool = coupons().await;

    let ids = ids_where(
        &pool,
        vec![
            condition("amount", FilterOperator::GreaterThan, 4_u32.into_value()),
            condition(
                "expires_at",
                FilterOperator::LessThan,
                expiry(2).into_value(),
            ),
            condition("is_active", FilterOperator::Equal, true.into_value()),
        ],
        by_amount(),
        (None, None),
    )
    .await;

    assert_eq!(ids, vec![1]);
}

// Sorting by a renamed field sorts by its column, and a page skips and takes
// rows.
#[tokio::test]
async fn sorting_and_paging_apply() {
    let pool = coupons().await;
    let descending = vec![SortKey {
        field: "amount",
        direction: SortDirectionModel::Descending,
    }];

    assert_eq!(
        ids_where(&pool, Vec::new(), descending.clone(), (None, None)).await,
        vec![2, 1, 3]
    );
    assert_eq!(
        ids_where(&pool, Vec::new(), descending, (Some(1), Some(2))).await,
        vec![3]
    );
}

// Assignments write typed values (and `NULL`s) to the columns.
#[tokio::test]
async fn assignments_write_typed_values() {
    let pool = coupons().await;

    let mut query_builder = QueryBuilder::<Sqlite>::new("UPDATE coupons SET ");
    let _ = Coupons::push_assignments(
        &mut query_builder,
        queryweave::__private::model_data::<Coupon, _>(vec![
            FieldValue {
                field: "label",
                value: Value::Text(None),
            },
            FieldValue {
                field: "amount",
                value: 42_u32.into_value(),
            },
            FieldValue {
                field: "expires_at",
                value: expiry(9).into_value(),
            },
        ]),
    );
    query_builder.push(" WHERE id = 3");
    query_builder.build().execute(&pool).await.unwrap();

    let (label, quantity, expires_at): (Option<String>, i64, DateTime<Utc>) =
        sqlx::query_as("SELECT label, quantity, expires_at FROM coupons WHERE id = 3")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((label, quantity, expires_at), (None, 42, expiry(9)));
}
