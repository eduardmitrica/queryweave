//! The generated code reaches sqlx through `queryweave_sql`, never by a
//! relative `sqlx::...` path: a local item named `sqlx` (or a renamed sqlx
//! dependency) does not break it.
use queryweave_derive::{AppendToQuery, AppendToQueryWithSeparator, Columns, Filters};

#[allow(dead_code)]
mod sqlx {
    pub struct QueryBuilder;
}

#[derive(Filters)]
pub struct OrderModel {
    #[filter(eq)]
    #[sortable]
    pub value: i32,
}

#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    pub value: i32,
}

#[derive(Debug, AppendToQueryWithSeparator)]
#[append_to_query(columns = Order)]
pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);

#[derive(Debug, AppendToQuery)]
#[append_to_query(columns = Order, filters = OrdersFilterOptionsModelWrapper)]
pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);

fn main() {
    let mut query = queryweave_sql::QueryBuilder::<::sqlx::Postgres>::new("SELECT * FROM orders ");
    let guard: Option<
        fn(&mut queryweave_sql::QueryBuilder<::sqlx::Postgres>, &OrdersQueryModelWrapper),
    > = None;
    OrdersQueryModelWrapper(OrdersQueryModel::default()).append_to_query(&mut query, guard);
}
