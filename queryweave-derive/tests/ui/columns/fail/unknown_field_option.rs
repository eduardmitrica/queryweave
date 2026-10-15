use queryweave_derive::Columns;

pub struct OrderModel {
    pub delivery_date: i64,
}

// `skip` is the only option of a field's `#[columns(...)]`.
#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    #[columns(hide)]
    pub delivery_date: i64,
}

fn main() {}
