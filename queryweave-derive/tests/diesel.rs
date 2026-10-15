//! The code `#[derive(Columns)]` generates for a Diesel entity
//! (`#[diesel(table_name)]`), run against a real SQLite (in memory) through
//! Diesel's own boxed queries and changesets.

use chrono::{DateTime, TimeZone, Utc};
use diesel::prelude::*;
use pretty_assertions::assert_eq;
use queryweave::{ChangeSet, ChangedFields, SortDirectionModel};
use queryweave_derive::{
    AppendConditions, AppendToQuery, Columns, Filters, MergePatch, MergePatchNullableFields,
};
use queryweave_sql::Junction;
use queryweave_sql::diesel::ValueMismatch;

/// The schema, as Diesel's CLI would print it.
mod schema {
    diesel::table! {
        orders (id) {
            id -> Binary,
            delivered_at -> TimestamptzSqlite,
            client -> Text,
        }
    }
}

use schema::orders;

#[allow(dead_code)]
#[derive(Filters)]
struct OrderModel {
    #[filter(lt, gt)]
    #[sortable]
    delivery_date: DateTime<Utc>,

    #[filter(sw, eq)]
    #[sortable]
    client: String,
}

/// The changes written back.
#[derive(Clone, MergePatch)]
#[merge_patch(patch = MergePatchClientModel)]
struct ClientModel {
    client: String,
    delivery_date: DateTime<Utc>,
}

#[derive(Default, MergePatchNullableFields)]
struct MergePatchClientModel {
    client: Option<String>,
    delivery_date: Option<DateTime<Utc>>,
}

/// The Diesel entity: `delivery_date` is stored in `delivered_at`, and the
/// binary id (a SQL type queryweave cannot convert) stores no domain field.
#[allow(dead_code)]
#[derive(Queryable, Selectable, Columns)]
#[diesel(table_name = orders)]
#[columns(model = OrderModel)]
struct Order {
    #[columns(skip)]
    id: Vec<u8>,
    #[domain_field(delivery_date)]
    delivered_at: DateTime<Utc>,
    client: String,
}

/// The wrappers writing a query model's data to the entity's boxed queries.
#[derive(AppendConditions)]
#[append_conditions(columns = Order)]
struct OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel);

#[derive(AppendToQuery)]
#[append_to_query(columns = Order, filters = OrdersFilterOptionsModelWrapper)]
struct OrdersQueryModelWrapper(OrdersQueryModel);

#[allow(
    clippy::unwrap_used,
    reason = "test setup: a failure here fails the test"
)]
fn day(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
}

/// A fresh in-memory database with three orders.
#[allow(
    clippy::unwrap_used,
    reason = "test setup: a failure here fails the test"
)]
fn orders() -> SqliteConnection {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();
    diesel::sql_query(
        "CREATE TABLE orders (id BLOB PRIMARY KEY, delivered_at TEXT NOT NULL, client TEXT NOT NULL)",
    )
    .execute(&mut connection)
    .unwrap();

    for (id, delivered, client) in [(1_u8, 1, "acme"), (2, 2, "acme corp"), (3, 3, "globex")] {
        diesel::insert_into(orders::table)
            .values((
                orders::id.eq(vec![id]),
                orders::delivered_at.eq(day(delivered)),
                orders::client.eq(client),
            ))
            .execute(&mut connection)
            .unwrap();
    }

    connection
}

/// The first byte of each id of `orders`.
fn ids(rows: Vec<Vec<u8>>) -> Vec<u8> {
    rows.into_iter()
        .filter_map(|id| id.first().copied())
        .collect()
}

// A query model's filters, sorting (by the renamed field's column) and page,
// through Diesel's boxed query.
#[test]
fn a_query_filters_sorts_and_pages_a_boxed_query() {
    let mut connection = orders();
    let query = OrdersQueryModel {
        page: Some(0),
        page_size: Some(2),
        sort: Some(vec![OrderSortDetailsModel {
            field: OrderSortFieldModel::DeliveryDate,
            order: SortDirectionModel::Descending,
        }]),
        and: OrdersFilterOptionsModel {
            client_sw: Some(String::from("acme")),
            delivery_date_gt: Some(day(1)),
            ..Default::default()
        },
    };

    let mut select = orders::table.select(orders::id).into_boxed();
    if let Some(condition) = Order::diesel_conditions(query.and.conditions(), Junction::Or).unwrap()
    {
        select = select.filter(condition);
    }
    for ordering in Order::diesel_order_by(query.sort_keys()) {
        select = select.then_order_by(ordering);
    }
    if let Some((limit, offset)) = queryweave_sql::diesel::page(query.page, query.page_size) {
        select = select.limit(limit).offset(offset);
    }

    // `client LIKE 'acme%' OR delivered_at > day 1`: every order, newest
    // first, the first page of two.
    assert_eq!(ids(select.load(&mut connection).unwrap()), vec![3, 2]);
}

// Without filters there is no condition at all.
#[test]
fn no_filters_make_no_condition() {
    let conditions = OrdersFilterOptionsModel::default().conditions();

    assert!(
        Order::diesel_conditions::<diesel::sqlite::Sqlite, _>(conditions, Junction::And)
            .unwrap()
            .is_none()
    );
}

// A merge patch's changed values set the changed columns only, through
// Diesel's own `update(..).set(..)`.
#[test]
fn changed_values_update_their_columns() {
    let mut connection = orders();
    let before = ClientModel {
        client: String::from("globex"),
        delivery_date: day(3),
    };
    let mut client = before.clone();
    client.merge(MergePatchClientModel {
        client: Some(String::from("initech")),
        ..Default::default()
    });
    let changed: ChangeSet = client.modified_fields(&before);

    let changeset = Order::diesel_changeset(client.changed_values(&changed)).unwrap();
    diesel::update(orders::table.filter(orders::id.eq(vec![3_u8])))
        .set(changeset)
        .execute(&mut connection)
        .unwrap();

    let row: (String, DateTime<Utc>) = orders::table
        .filter(orders::id.eq(vec![3_u8]))
        .select((orders::client, orders::delivered_at))
        .first(&mut connection)
        .unwrap();
    assert_eq!(row, (String::from("initech"), day(3)));
}

// A value that does not fit its column is an error, not a panic or a query.
#[test]
fn a_value_that_does_not_fit_is_reported() {
    let mismatched = queryweave::__private::model_data::<OrdersFilterOptionsModel, _>(vec![
        queryweave::FilterCondition {
            field: "client",
            operator: queryweave::FilterOperator::Equal,
            value: queryweave::Value::I32(Some(1)),
        },
    ]);

    assert_eq!(
        Order::diesel_conditions::<diesel::sqlite::Sqlite, _>(mismatched, Junction::And).err(),
        Some(ValueMismatch { field: "client" })
    );
}

// The query wrapper applies a query model's filters, sorting and page to a
// boxed query in one call, after the query's own `select` and `filter` (here
// a privileged guard leaving out globex's orders).
#[test]
fn the_query_wrapper_applies_a_query_to_a_boxed_query() {
    let mut connection = orders();
    let query = OrdersQueryModelWrapper(OrdersQueryModel {
        page: Some(0),
        page_size: Some(2),
        sort: Some(vec![OrderSortDetailsModel {
            field: OrderSortFieldModel::DeliveryDate,
            order: SortDirectionModel::Ascending,
        }]),
        and: OrdersFilterOptionsModel {
            delivery_date_gt: Some(day(1)),
            ..Default::default()
        },
    });
    let guarded = orders::table
        .select(orders::id)
        .filter(orders::client.ne("globex"))
        .into_boxed();

    let select = query.diesel_apply_to(guarded).unwrap();

    // Delivered after day 1 and not globex's: only order 2.
    assert_eq!(ids(select.load(&mut connection).unwrap()), vec![2]);
}

// The filter-options wrapper gives the set filters as one condition, joined as
// asked, for the caller's own `filter` or `or_filter`.
#[test]
fn the_filters_wrapper_gives_one_condition() {
    let mut connection = orders();
    let filters = OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel {
        client_eq: Some(String::from("globex")),
        delivery_date_lt: Some(day(2)),
        ..Default::default()
    });

    let condition = filters.diesel_condition(Junction::Or).unwrap().unwrap();
    let select = orders::table
        .select(orders::id)
        .order_by(orders::id)
        .into_boxed()
        .filter(condition);

    // globex's order, or one delivered before day 2.
    assert_eq!(ids(select.load(&mut connection).unwrap()), vec![1, 3]);
    assert!(
        OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel::default())
            .diesel_condition::<diesel::sqlite::Sqlite>(Junction::And)
            .unwrap()
            .is_none()
    );
}
