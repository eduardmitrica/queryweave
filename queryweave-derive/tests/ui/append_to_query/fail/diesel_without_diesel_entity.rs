use queryweave_derive::{AppendToQuery, AppendConditions, Columns, Filters};

mod schema {
    diesel::table! {
        orders (id) {
            id -> Integer,
            client -> Text,
        }
    }
}

use schema::orders;

#[allow(dead_code)]
#[derive(Filters)]
struct OrderModel {
    #[filter(eq)]
    #[sortable]
    client: String,
}

// An sqlx entity: no `#[diesel(table_name = ..)]`, so no Diesel methods.
#[allow(dead_code)]
#[derive(Columns)]
#[columns(model = OrderModel)]
struct Order {
    client: String,
}

#[derive(AppendConditions)]
#[append_conditions(columns = Order)]
struct OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel);

#[derive(AppendToQuery)]
#[append_to_query(columns = Order, filters = OrdersFilterOptionsModelWrapper)]
struct OrdersQueryModelWrapper(OrdersQueryModel);

fn main() {
    use diesel::QueryDsl;

    // The wrapper's Diesel method needs a Diesel entity.
    let _ = OrdersQueryModelWrapper(OrdersQueryModel::default())
        .diesel_apply_to::<diesel::sqlite::Sqlite, _>(orders::table.into_boxed());
}
