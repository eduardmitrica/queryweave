use chrono::{DateTime, Utc};
use queryweave_derive::{AppendToQuery, Columns, Filters};

#[derive(Filters)]
pub struct OrderModel {
    #[filter(lt, eq, gt)]
    #[sortable]
    pub delivery_date: DateTime<Utc>,
}

#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    pub delivery_date: DateTime<Utc>,
}

#[derive(Debug, AppendToQuery)]
#[append_to_query(columns = Order)]
pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);

fn main() {}
