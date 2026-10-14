//! Behavior of the code `#[derive(AppendToQuery)]` and
//! `#[derive(AppendToQueryWithSeparator)]` generate.

use chrono::{DateTime, Utc};
use pretty_assertions::assert_eq;
use queryweave::SortDirectionModel;
use queryweave_derive::{AppendToQuery, AppendToQueryWithSeparator, Columns, Filters};
use sqlx::{MySql, Postgres, QueryBuilder};

#[allow(dead_code)]
#[derive(Filters)]
struct OrderModel {
    #[filter(lt, gt)]
    #[sortable]
    delivery_date: DateTime<Utc>,

    #[filter(sw, eq)]
    client: String,
}

// `delivery_date` is stored in the `delivered_at` column.
#[allow(dead_code)]
#[derive(Columns)]
#[columns(model = OrderModel)]
struct Order {
    #[domain_field(delivery_date)]
    delivered_at: DateTime<Utc>,
    client: String,
}

#[derive(Debug, AppendToQueryWithSeparator)]
#[append_to_query(columns = Order)]
struct OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel);

#[derive(Debug, AppendToQuery)]
#[append_to_query(columns = Order, filters = OrdersFilterOptionsModelWrapper)]
struct OrdersQueryModelWrapper(OrdersQueryModel);

type NoGuard<DB> = Option<fn(&mut QueryBuilder<'_, DB>, &OrdersQueryModelWrapper)>;

fn query() -> OrdersQueryModel {
    OrdersQueryModel {
        page: Some(2),
        page_size: Some(10),
        sort: Some(vec![OrderSortDetailsModel {
            field: OrderSortFieldModel::DeliveryDate,
            order: SortDirectionModel::Descending,
        }]),
        and: OrdersFilterOptionsModel {
            client_sw: Some(String::from("ab")),
            ..Default::default()
        },
    }
}

// Filters, sorting (by the renamed field's column) and the page, in order,
// through the inherent method (no trait import).
#[test]
fn a_query_appends_its_filters_sorting_and_page() {
    let mut query_builder = QueryBuilder::<Postgres>::new("SELECT * FROM orders ");
    OrdersQueryModelWrapper(query()).append_to_query(&mut query_builder, NoGuard::<Postgres>::None);

    assert_eq!(
        query_builder.sql(),
        "SELECT * FROM orders WHERE client LIKE $1 ESCAPE '!' ORDER BY delivered_at DESC LIMIT $2 OFFSET $3"
    );
}

// The privileged guard runs between the filters and the sorting.
#[test]
fn the_privileged_guard_runs_after_the_filters() {
    let mut query_builder = QueryBuilder::<Postgres>::new("SELECT * FROM orders ");
    OrdersQueryModelWrapper(query()).append_to_query(
        &mut query_builder,
        Some(
            |builder: &mut QueryBuilder<'_, Postgres>, _: &OrdersQueryModelWrapper| {
                builder.push("AND is_visible ");
            },
        ),
    );

    assert_eq!(
        query_builder.sql(),
        "SELECT * FROM orders WHERE client LIKE $1 ESCAPE '!' AND is_visible ORDER BY delivered_at DESC LIMIT $2 OFFSET $3"
    );
}

// Without filters there is no `WHERE`; the same derive writes for MySQL.
#[test]
fn a_query_without_filters_has_no_where_clause_on_any_database() {
    let mut query_builder = QueryBuilder::<MySql>::new("SELECT * FROM orders ");
    OrdersQueryModelWrapper(OrdersQueryModel {
        and: OrdersFilterOptionsModel::default(),
        ..query()
    })
    .append_to_query(&mut query_builder, NoGuard::<MySql>::None);

    assert_eq!(
        query_builder.sql(),
        "SELECT * FROM orders ORDER BY delivered_at DESC LIMIT ? OFFSET ?"
    );
}

// The filter options join their conditions with the given junction.
#[test]
fn filter_options_are_joined_by_the_junction() {
    let mut query_builder = QueryBuilder::<Postgres>::new("");
    OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel {
        delivery_date_gt: Some(DateTime::<Utc>::UNIX_EPOCH),
        client_eq: Some(String::from("acme")),
        ..Default::default()
    })
    .append_to_query(&mut query_builder, queryweave_sql::Junction::Or);

    assert_eq!(query_builder.sql(), "delivered_at > $1 OR client = $2 ");
}
