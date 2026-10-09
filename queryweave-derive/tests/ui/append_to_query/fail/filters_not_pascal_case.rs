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

// The wrapper must be named in PascalCase; the error points at the path
// naming it.
#[derive(Debug, AppendToQuery)]
#[append_to_query(columns = Order, filters = orders_filter_options_model_wrapper)]
pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);

fn main() {}
