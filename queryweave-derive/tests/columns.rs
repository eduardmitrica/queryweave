//! Behavior of the code `#[derive(Columns)]` generates.

use pretty_assertions::assert_eq;
use queryweave::{ChangeSet, ChangedFields};
use queryweave_derive::{Columns, MergePatch, MergePatchNullableFields};
use sqlx::{Postgres, QueryBuilder};

/// A domain model whose changes are written through `Order`'s columns; the
/// derive builds its data, as in application code.
#[derive(Clone, MergePatch)]
#[merge_patch(patch = MergePatchOrderModel)]
struct OrderModel {
    number: String,
    delivery_date: i64,
}

#[derive(Default, MergePatchNullableFields)]
struct MergePatchOrderModel {
    number: Option<String>,
    delivery_date: Option<i64>,
}

// One column renamed by `sqlx`, one storing a differently named domain field,
// and one that is not a column at all.
#[allow(dead_code)]
#[derive(sqlx::FromRow, Columns)]
#[columns(model = OrderModel)]
struct Order {
    #[sqlx(rename = "order_number")]
    number: String,
    #[domain_field(delivery_date)]
    delivered_at: i64,
    #[sqlx(skip)]
    cached: bool,
}

#[test]
fn the_entity_maps_each_domain_field_onto_its_column() {
    use queryweave_sql::Columns as _;

    assert_eq!(Order::NAME, "Order");
    assert_eq!(
        Order::COLUMNS,
        &[
            ("number", "order_number"),
            ("delivery_date", "delivered_at")
        ]
    );
}

// The SQL methods are inherent, usable without importing the trait, and write
// the entity's columns.
#[test]
fn the_generated_methods_write_the_entitys_columns() {
    let before = OrderModel {
        number: String::from("A-1"),
        delivery_date: 1,
    };
    let mut order = before.clone();
    order.merge(MergePatchOrderModel {
        number: Some(String::from("A-2")),
        delivery_date: Some(2),
    });
    let changed = order.modified_fields(&before);

    let mut query_builder = QueryBuilder::<Postgres>::new("UPDATE orders SET ");
    let _ = Order::push_assignments(&mut query_builder, order.changed_values(&changed));

    assert_eq!(
        query_builder.sql(),
        "UPDATE orders SET order_number = $1, delivered_at = $2"
    );
}

// Unchanged fields are not written.
#[test]
fn only_the_changed_fields_are_written() {
    let order = OrderModel {
        number: String::from("A-1"),
        delivery_date: 1,
    };
    let only_the_number = ChangeSet::from_iter(["number"]);

    let mut query_builder = QueryBuilder::<Postgres>::new("UPDATE orders SET ");
    let _ = Order::push_assignments(&mut query_builder, order.changed_values(&only_the_number));

    assert_eq!(query_builder.sql(), "UPDATE orders SET order_number = $1");
}
