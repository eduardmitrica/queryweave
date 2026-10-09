#![allow(clippy::expect_used)]

//! The SQL written by `queryweave_sql` against real MySQL and MariaDB
//! servers, in containers, so these tests need Docker (Postgres is
//! `tests/postgres.rs`, SQLite `tests/sqlite.rs`). Same scenario on both: `LIKE`
//! metacharacters match literally, typed values compare against their
//! columns, sorting and paging apply, and assignments write typed values.

use chrono::{DateTime, TimeZone, Utc};
use pretty_assertions::assert_eq;
use queryweave::{
    FieldValue, FilterCondition, FilterOperator, IntoValue, PersistedFields, SortDirectionModel,
    SortKey, Value,
};
use queryweave_sql::{Columns, push_page};
use rstest::rstest;
use sqlx::{MySql, MySqlPool, QueryBuilder};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

/// How long a server may take to start, pulling its image included.
const STARTUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
const PORT: u16 = 3306;
const PASSWORD: &str = "secret";
const DATABASE: &str = "coupons";

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

/// A MySQL-protocol server (`image:tag`) with an empty `coupons` database.
///
/// Both images run a temporary server without networking while they
/// initialize, then the real one; only the latter listens on the port.
async fn server(image: &str, tag: &str, root_password_env: &str) -> ContainerAsync<GenericImage> {
    GenericImage::new(image, tag)
        .with_exposed_port(PORT.tcp())
        .with_wait_for(WaitFor::message_on_either_std(format!("port: {PORT}")))
        .with_env_var(root_password_env, PASSWORD)
        .with_env_var(
            if image == "mysql" {
                "MYSQL_DATABASE"
            } else {
                "MARIADB_DATABASE"
            },
            DATABASE,
        )
        .with_startup_timeout(STARTUP_TIMEOUT)
        .start()
        .await
        .expect("Failed to start the database server")
}

fn expiry(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0)
        .single()
        .expect("A valid date")
}

/// Three coupons in a fresh table.
async fn coupons(container: &ContainerAsync<GenericImage>) -> MySqlPool {
    let host = container.get_host().await.expect("Failed to get the host");
    let port = container
        .get_host_port_ipv4(PORT)
        .await
        .expect("Failed to get the mapped port");
    let pool = MySqlPool::connect(&format!("mysql://root:{PASSWORD}@{host}:{port}/{DATABASE}"))
        .await
        .expect("Failed to connect to the database");

    sqlx::query(
        "CREATE TABLE coupons (
            id BIGINT PRIMARY KEY,
            label VARCHAR(64) NULL,
            quantity INT NOT NULL,
            expires_at DATETIME NOT NULL,
            is_active BOOLEAN NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create the table");

    for (id, label, quantity, day, is_active) in [
        (1_i64, "50%_off!", 5_i32, 1, true),
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
            .expect("Failed to insert a coupon");
    }

    pool
}

async fn ids_where(
    pool: &MySqlPool,
    conditions: Vec<FilterCondition>,
    sort_keys: Vec<SortKey>,
    page: (Option<u32>, Option<u32>),
) -> Vec<i64> {
    let mut query_builder = QueryBuilder::<MySql>::new("SELECT id FROM coupons ");
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
        .expect("Failed to select coupons")
}

const fn condition(field: &'static str, operator: FilterOperator, value: Value) -> FilterCondition {
    FilterCondition {
        field,
        operator,
        value,
    }
}

fn amount(direction: SortDirectionModel) -> Vec<SortKey> {
    vec![SortKey {
        field: "amount",
        direction,
    }]
}

#[rstest]
#[case::mysql("mysql", "8.4", "MYSQL_ROOT_PASSWORD")]
#[case::mariadb("mariadb", "11.4", "MARIADB_ROOT_PASSWORD")]
#[tokio::test]
async fn test_the_sql_runs_on_mysql_protocol_databases(
    #[case] image: &str,
    #[case] tag: &str,
    #[case] root_password_env: &str,
) {
    let container = server(image, tag, root_password_env).await;
    let pool = coupons(&container).await;

    // `%`, `_` and the escape character `!` match literally.
    let literal = ids_where(
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

    // A widened `u32`, a date-time and a boolean compare against their columns.
    let typed = ids_where(
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
        amount(SortDirectionModel::Ascending),
        (None, None),
    )
    .await;

    // Sorting by a renamed field sorts by its column; a page skips and takes.
    let sorted = ids_where(
        &pool,
        Vec::new(),
        amount(SortDirectionModel::Descending),
        (None, None),
    )
    .await;
    let paged = ids_where(
        &pool,
        Vec::new(),
        amount(SortDirectionModel::Descending),
        (Some(1), Some(2)),
    )
    .await;

    // Assignments write typed values (and `NULL`s).
    let mut query_builder = QueryBuilder::<MySql>::new("UPDATE coupons SET ");
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
    query_builder
        .build()
        .execute(&pool)
        .await
        .expect("Failed to update a coupon");
    let written: (Option<String>, i32, DateTime<Utc>) =
        sqlx::query_as("SELECT label, quantity, expires_at FROM coupons WHERE id = 3")
            .fetch_one(&pool)
            .await
            .expect("Failed to read the coupon back");

    // Closed before asserting, so it is closed even when an assertion fails.
    pool.close().await;

    assert_eq!(literal, vec![1]);
    assert_eq!(typed, vec![1]);
    assert_eq!(sorted, vec![2, 1, 3]);
    assert_eq!(paged, vec![3]);
    assert_eq!(written, (None, 42, expiry(9)));
}
