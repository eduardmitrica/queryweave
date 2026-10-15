use queryweave_derive::{Columns, Filters};

mod schema {
    diesel::table! {
        orders (id) {
            id -> Binary,
            client -> Text,
        }
    }
}

use schema::orders;

#[allow(dead_code)]
#[derive(Filters)]
struct OrderModel {
    #[filter(eq)]
    client: String,
}

// The binary `id` takes no domain value, and is not marked
// `#[columns(skip)]`: the conditions cannot be written.
#[allow(dead_code)]
#[derive(diesel::Queryable, diesel::Selectable, Columns)]
#[diesel(table_name = orders)]
#[columns(model = OrderModel)]
struct Order {
    id: Vec<u8>,
    client: String,
}

fn main() {
    let _ = Order::diesel_conditions::<diesel::sqlite::Sqlite, _>(
        OrdersFilterOptionsModel::default().conditions(),
        queryweave_sql::Junction::And,
    );
}
