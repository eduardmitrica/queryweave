use chrono::{DateTime, Utc};
use queryweave_derive::{AppendToQueryWithSeparator, Filters};

#[derive(Filters)]
pub struct OrderModel {
    #[filter(lt)]
    pub delivery_date: DateTime<Utc>,
}

// Without `#[append_to_query(columns = Entity)]` there is no telling which columns store the
// filtered fields.
#[derive(Debug, AppendToQueryWithSeparator)]
pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);

fn main() {}
