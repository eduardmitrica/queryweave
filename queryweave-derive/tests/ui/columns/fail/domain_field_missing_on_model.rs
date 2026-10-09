use queryweave_derive::Columns;

pub struct OrderModel {
    pub delivery_date: i64,
}

// `delivery_dat` is not a field of `OrderModel`.
#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    #[domain_field(delivery_dat)]
    pub delivered_at: i64,
}

fn main() {}
