use queryweave_derive::Columns;

// Without `#[columns(model = Model)]` there is no telling which domain model's
// fields the columns store.
#[derive(Columns)]
pub struct Order {
    pub delivered_at: i64,
}

fn main() {}
