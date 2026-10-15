use queryweave_derive::{AppendToQuery, AppendToQueryWithSeparator, Columns, Filters};

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

#[allow(dead_code)]
#[derive(diesel::Queryable, diesel::Selectable, Columns)]
#[diesel(table_name = orders)]
#[columns(model = OrderModel)]
struct Order {
    #[columns(skip)]
    id: i32,
    client: String,
}

#[derive(AppendToQueryWithSeparator)]
#[append_to_query(columns = Order)]
struct OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel);

#[derive(AppendToQuery)]
#[append_to_query(columns = Order, filters = OrdersFilterOptionsModelWrapper)]
struct OrdersQueryModelWrapper(OrdersQueryModel);

fn main() {
    // The query is not boxed: its type would change with each clause.
    let _ = OrdersQueryModelWrapper(OrdersQueryModel::default())
        .diesel_apply_to::<diesel::sqlite::Sqlite, _>(orders::table);
}
