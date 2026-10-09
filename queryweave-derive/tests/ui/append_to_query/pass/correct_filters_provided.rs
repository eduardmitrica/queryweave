use chrono::{DateTime, Utc};
use queryweave_derive::{AppendToQuery, AppendToQueryWithSeparator, Columns, Filters};

#[derive(Filters)]
pub struct OrderModel {
    #[filter(lt, eq, gt)]
    #[sortable]
    pub delivery_date: DateTime<Utc>,
}

// The domain field is stored in a differently named column.
#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    #[domain_field(delivery_date)]
    pub delivered_at: DateTime<Utc>,
}

#[derive(Debug, AppendToQueryWithSeparator)]
#[append_to_query(columns = Order)]
pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);

#[derive(Debug, AppendToQuery)]
#[append_to_query(columns = Order, filters = OrdersFilterOptionsModelWrapper)]
pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);

fn main() {}
