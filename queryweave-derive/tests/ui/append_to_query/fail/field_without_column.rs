use chrono::{DateTime, Utc};
use queryweave_derive::{AppendToQuery, AppendConditions, Columns, Filters};

#[derive(Filters)]
pub struct OrderModel {
    #[filter(eq)]
    pub number: String,

    #[filter(lt)]
    #[sortable]
    pub delivery_date: DateTime<Utc>,
}

// The table has no column storing `delivery_date` (it is stored in
// `delivered_at`, but nothing says so): the query could not be built, so it
// does not compile.
#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    pub number: String,
    pub delivered_at: DateTime<Utc>,
}

#[derive(Debug, AppendConditions)]
#[append_conditions(columns = Order)]
pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);

#[derive(Debug, AppendToQuery)]
#[append_to_query(columns = Order, filters = OrdersFilterOptionsModelWrapper)]
pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);

fn main() {}
