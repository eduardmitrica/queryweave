use chrono::{DateTime, Utc};
use queryweave_derive::{AppendConditions, Filters};

#[derive(Filters)]
pub struct OrderModel {
    #[filter(lt)]
    pub delivery_date: DateTime<Utc>,
}

// Without `#[append_conditions(columns = Entity)]` there is no telling which columns store the
// filtered fields.
#[derive(Debug, AppendConditions)]
pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);

fn main() {}
