use queryweave_derive::Columns;

pub struct OrderModel {
    pub delivery_date: i64,
}

// A skipped column stores no domain field, so it cannot name one.
#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    #[columns(skip)]
    #[domain_field(delivery_date)]
    pub delivered_at: i64,
}

fn main() {}
