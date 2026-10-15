#![allow(clippy::expect_used)]

//! The `queryweave_sql::diesel` backend against a real Postgres, in a
//! container, so these tests need Docker: the same cases as `diesel_sqlite.rs`,
//! through Diesel's boxed queries and changesets, run by `diesel-async` (pure
//! Rust, so no libpq is needed). Postgres checks parameter types strictly, so
//! this is where a value bound with the wrong SQL type would fail.
//!
//! `Coupons` stands in for the code `Columns` generates for a Diesel entity:
//! a `match` from each domain field to its typed column.

use chrono::{DateTime, TimeZone, Utc};
use diesel::pg::Pg;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use pretty_assertions::assert_eq;
use queryweave::{
    FieldValue, FilterCondition, FilterOperator, IntoValue, SortDirectionModel, SortKey, Value,
};
use queryweave_sql::Junction;
use queryweave_sql::diesel::{
    BoxedCondition, BoxedOrdering, ValueMismatch, ValueOf, apply, assignment, condition, join,
    ordering, page,
};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

/// How long the server may take to start, pulling its image included.
const STARTUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
const PORT: u16 = 5432;
const PASSWORD: &str = "postgres";

/// The schema, as Diesel's CLI would print it.
mod schema {
    diesel::table! {
        coupons (id) {
            id -> Integer,
            label -> Nullable<Text>,
            quantity -> Integer,
            expires_at -> Timestamptz,
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
    ) -> Result<BoxedCondition<'static, coupons::table, Pg>, ValueMismatch> {
        match filter_condition.field {
            "label" => condition(coupons::label, filter_condition),
            "amount" => condition(coupons::quantity, filter_condition),
            "expires_at" => condition(coupons::expires_at, filter_condition),
            "is_active" => condition(coupons::is_active, filter_condition),
            field => Err(ValueMismatch { field }),
        }
    }

    fn ordering(sort_key: SortKey) -> Option<BoxedOrdering<'static, coupons::table, Pg>> {
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
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0)
        .single()
        .expect("A valid date")
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

/// Three coupons in a fresh table.
///
/// The image's entrypoint starts a temporary server to initialize the
/// database before the real one, both logging that they are ready: connecting
/// is retried until the real one accepts.
async fn coupons(container: &ContainerAsync<GenericImage>) -> AsyncPgConnection {
    let host = container.get_host().await.expect("Failed to get the host");
    let port = container
        .get_host_port_ipv4(PORT)
        .await
        .expect("Failed to get the mapped port");
    let url = format!("postgres://postgres:{PASSWORD}@{host}:{port}/postgres");
    let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;
    let mut connection = loop {
        match AsyncPgConnection::establish(&url).await {
            Ok(connection) => break connection,
            Err(error) => {
                assert!(
                    std::time::Instant::now() < deadline,
                    "Postgres did not accept connections: {error}"
                );
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            },
        }
    };

    diesel::sql_query(
        "CREATE TABLE coupons (
            id INTEGER PRIMARY KEY,
            label TEXT,
            quantity INTEGER NOT NULL,
            expires_at TIMESTAMPTZ NOT NULL,
            is_active BOOLEAN NOT NULL
        )",
    )
    .execute(&mut connection)
    .await
    .expect("Failed to create the table");

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
            .await
            .expect("Failed to insert a coupon");
    }

    connection
}

/// The ids of the coupons matching `conditions` (joined by `junction`),
/// sorted by `sort_keys` and paged.
async fn ids_where(
    connection: &mut AsyncPgConnection,
    conditions: Vec<FilterCondition>,
    junction: Junction,
    sort_keys: Vec<SortKey>,
    page_request: (Option<u32>, Option<u32>),
) -> Vec<i32> {
    let conditions = conditions
        .into_iter()
        .map(Coupons::condition)
        .collect::<Result<Vec<_>, _>>()
        .expect("Every value fits its column");
    let orderings = sort_keys
        .into_iter()
        .map(|sort_key| Coupons::ordering(sort_key).expect("A known field"))
        .collect();

    apply(
        coupons::table.select(coupons::id).into_boxed::<Pg>(),
        join(conditions, junction),
        orderings,
        page(page_request.0, page_request.1),
    )
    .load(connection)
    .await
    .expect("Failed to select coupons")
}

const fn filter(field: &'static str, operator: FilterOperator, value: Value) -> FilterCondition {
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

/// `%`, `_` and the escape character `!` match literally, with the escape
/// character bound (`ESCAPE $2`).
async fn like_metacharacters_match_literally(connection: &mut AsyncPgConnection) {
    for (operator, needle, expected) in [
        (FilterOperator::Contains, "%_off!", vec![1]),
        (FilterOperator::StartsWith, "50%", vec![1]),
        (FilterOperator::EndsWith, " off", vec![2]),
    ] {
        let ids = ids_where(
            connection,
            vec![filter("label", operator, needle.to_owned().into_value())],
            Junction::And,
            Vec::new(),
            (None, None),
        )
        .await;
        assert_eq!(ids, expected, "{operator:?} {needle:?}");
    }
}

/// Widened integers (a `u8` as `smallint`, a `u16` as `integer`), a UTC
/// date-time and a boolean compare against their columns.
async fn typed_values_compare_against_their_columns(connection: &mut AsyncPgConnection) {
    let ids = ids_where(
        connection,
        vec![
            filter("amount", FilterOperator::GreaterThan, 4_u8.into_value()),
            filter("amount", FilterOperator::LessThan, 6_u16.into_value()),
            filter(
                "expires_at",
                FilterOperator::LessThan,
                expiry(2).into_value(),
            ),
            filter("is_active", FilterOperator::Equal, true.into_value()),
        ],
        Junction::And,
        Vec::new(),
        (None, None),
    )
    .await;

    assert_eq!(ids, vec![1]);
}

/// Conditions joined by `OR`, sorted by the renamed field's column; then
/// sorting alone, and a page.
async fn junctions_sorting_and_pages_apply(connection: &mut AsyncPgConnection) {
    let either = ids_where(
        connection,
        vec![
            filter(
                "label",
                FilterOperator::Equal,
                "plain".to_owned().into_value(),
            ),
            filter("amount", FilterOperator::Equal, 7_i32.into_value()),
        ],
        Junction::Or,
        amount(SortDirectionModel::Ascending),
        (None, None),
    )
    .await;
    let sorted = ids_where(
        connection,
        Vec::new(),
        Junction::And,
        amount(SortDirectionModel::Descending),
        (None, None),
    )
    .await;
    let paged = ids_where(
        connection,
        Vec::new(),
        Junction::And,
        amount(SortDirectionModel::Descending),
        (Some(1), Some(2)),
    )
    .await;

    assert_eq!(
        (either, sorted, paged),
        (vec![3, 2], vec![2, 1, 3], vec![3])
    );
}

/// A changeset writes typed values, and a `NULL`, through Diesel's own
/// `update(..).set(..)`.
async fn assignments_write_typed_values(connection: &mut AsyncPgConnection) {
    let changeset = Coupons::changeset(vec![
        FieldValue {
            field: "label",
            value: Value::Text(None),
        },
        FieldValue {
            field: "amount",
            value: 42_u16.into_value(),
        },
        FieldValue {
            field: "expires_at",
            value: expiry(9).into_value(),
        },
        FieldValue {
            field: "is_active",
            value: true.into_value(),
        },
    ])
    .expect("Every value fits its column");
    diesel::update(coupons::table.filter(coupons::id.eq(3)))
        .set(changeset)
        .execute(connection)
        .await
        .expect("Failed to update the coupon");

    let row: (Option<String>, i32, DateTime<Utc>, bool) = coupons::table
        .filter(coupons::id.eq(3))
        .select((
            coupons::label,
            coupons::quantity,
            coupons::expires_at,
            coupons::is_active,
        ))
        .first(connection)
        .await
        .expect("Failed to read the coupon");
    assert_eq!(row, (None, 42, expiry(9), true));
}

#[tokio::test]
async fn test_the_diesel_backend_runs_on_postgres() {
    let container = server().await;
    let mut connection = coupons(&container).await;

    like_metacharacters_match_literally(&mut connection).await;
    typed_values_compare_against_their_columns(&mut connection).await;
    junctions_sorting_and_pages_apply(&mut connection).await;
    assignments_write_typed_values(&mut connection).await;
}
