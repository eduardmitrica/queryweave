use chrono::{DateTime, Utc};
use queryweave_derive::{AppendConditions, Columns, Filters};

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

#[derive(Debug, AppendConditions)]
#[append_conditions(columns = Order)]
pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);

fn main() {}
