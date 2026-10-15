use queryweave_derive::{Columns, Filters};

mod schema {
    diesel::table! {
        orders (id) {
            id -> Integer,
            quantity -> Text,
        }
    }
}

use schema::orders;

#[allow(dead_code)]
#[derive(Filters)]
struct OrderModel {
    #[filter(eq)]
    quantity: u8,
}

// The domain's `quantity` is a number, its column text: filtering on it fails
// to build where the conditions are written.
#[allow(dead_code)]
#[derive(diesel::Queryable, diesel::Selectable, Columns)]
#[diesel(table_name = orders)]
#[columns(model = OrderModel)]
struct Order {
    #[columns(skip)]
    id: i32,
    quantity: String,
}

fn main() {
    let _ = Order::diesel_conditions::<diesel::sqlite::Sqlite, _>(
        OrdersFilterOptionsModel::default().conditions(),
        queryweave_sql::Junction::And,
    );
}
