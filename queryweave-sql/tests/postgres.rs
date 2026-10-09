#![allow(clippy::expect_used)]

//! The SQL written by `queryweave_sql` against a real Postgres, in a
//! container, so these tests need Docker.
//!
//! A `u32` domain value has no Postgres type of its own, so it is bound as a
//! `bigint` (see `queryweave::IntoValue`). These tests check that this works on
//! a narrower `integer` column, for both comparisons and writes. They also
//! check the field → column mapping end to end: the domain field is called
//! `amount` (on the `Amount` model below), and the `Columns` of `Orders` store
//! it in the `capacity` column, which is what the SQL must name.

use pretty_assertions::assert_eq;
use queryweave::{FieldValue, FilterCondition, FilterOperator, IntoValue, PersistedFields};
use queryweave_sql::Columns;
use sqlx::{PgPool, Postgres, QueryBuilder};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

/// How long the server may take to start, pulling its image included.
const STARTUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
const PORT: u16 = 5432;
const PASSWORD: &str = "postgres";

/// A domain model with a single field, `amount`.
struct Amount;

impl PersistedFields for Amount {
    const FIELD_NAMES: &'static [&'static str] = &["amount"];
}

/// The `orders` table, storing a domain field named differently from its
/// column: `amount` in `capacity`.
struct Orders;

impl Columns for Orders {
    const NAME: &'static str = "Orders";
    const COLUMNS: &'static [(&'static str, &'static str)] = &[("amount", "capacity")];
}

/// A Postgres server.
async fn server() -> ContainerAsync<GenericImage> {
    GenericImage::new("postgres", "18.6")
        .with_exposed_port(PORT.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", PASSWORD)
        .with_startup_timeout(STARTUP_TIMEOUT)
        .start()
        .await
        .expect("Failed to start the database server")
}

/// One order, of capacity 12, in a fresh `orders` table with an `integer`
/// `capacity`.
///
/// The image's entrypoint starts a temporary server to initialize the
/// database before the real one, both logging that they are ready: connecting
/// is retried until the real one accepts.
async fn orders(container: &ContainerAsync<GenericImage>) -> PgPool {
    let host = container.get_host().await.expect("Failed to get the host");
    let port = container
        .get_host_port_ipv4(PORT)
        .await
        .expect("Failed to get the mapped port");
    let url = format!("postgres://postgres:{PASSWORD}@{host}:{port}/postgres");
    let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;
    let pool = loop {
        match PgPool::connect(&url).await {
            Ok(pool) => break pool,
            Err(error) => {
                assert!(
                    std::time::Instant::now() < deadline,
                    "Postgres did not accept connections: {error}"
                );
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            },
        }
    };

    sqlx::query("CREATE TABLE orders (id INTEGER PRIMARY KEY, capacity INTEGER NOT NULL)")
        .execute(&pool)
        .await
        .expect("Failed to create the table");
    sqlx::query("INSERT INTO orders VALUES (1, 12)")
        .execute(&pool)
        .await
        .expect("Failed to insert an order");

    pool
}

async fn count_orders_where(pool: &PgPool, condition: FilterCondition) -> i64 {
    let mut query_builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM orders WHERE ");
    Orders::push_conditions(
        &mut query_builder,
        queryweave::__private::model_data::<Amount, _>(vec![condition]),
        "AND",
    );

    query_builder
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await
        .expect("Failed to count orders")
}

async fn set_capacity(pool: &PgPool, capacity: u32) -> Result<i32, sqlx::Error> {
    let mut query_builder = QueryBuilder::<Postgres>::new("UPDATE orders SET ");
    // Nothing else goes in the `SET` list, so the returned builder is not kept.
    let _ = Orders::push_assignments(
        &mut query_builder,
        queryweave::__private::model_data::<Amount, _>(vec![FieldValue {
            field: "amount",
            value: capacity.into_value(),
        }]),
    );
    query_builder.push(" WHERE id = 1 RETURNING capacity");

    query_builder
        .build_query_scalar::<i32>()
        .fetch_one(pool)
        .await
}

// Postgres compares an `integer` column with a `bigint` parameter, and a
// value beyond the column's range compares correctly instead of overflowing.
#[tokio::test]
async fn test_a_u32_filter_compares_against_an_integer_column() {
    let container = server().await;
    let pool = orders(&container).await;

    let mut counts = Vec::new();
    for (operator, capacity) in [
        (FilterOperator::LessThan, 13),
        (FilterOperator::LessThan, 12),
        (FilterOperator::Equal, 12),
        (FilterOperator::GreaterThan, 11),
        (FilterOperator::LessThan, u32::MAX),
    ] {
        let condition = FilterCondition {
            field: "amount",
            operator,
            value: capacity.into_value(),
        };
        counts.push(count_orders_where(&pool, condition).await);
    }
    // Closed before asserting, so it is closed even when the assertion fails.
    pool.close().await;

    assert_eq!(counts, [1, 0, 1, 1, 1]);
}

// Postgres assignment-casts a `bigint` parameter into an `integer` column,
// and rejects a value out of the column's range instead of truncating it.
#[tokio::test]
async fn test_a_u32_value_is_written_to_an_integer_column() {
    let container = server().await;
    let pool = orders(&container).await;

    let in_range = set_capacity(&pool, 20).await;
    let out_of_range = set_capacity(&pool, u32::MAX).await;
    // Closed before asserting, so it is closed even when an assertion fails.
    pool.close().await;

    assert_eq!(in_range.ok(), Some(20));
    let error = out_of_range.expect_err("An out-of-range write must fail");
    assert!(
        error.to_string().contains("out of range"),
        "Unexpected error: {error}"
    );
}
