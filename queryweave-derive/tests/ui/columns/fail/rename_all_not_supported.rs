use queryweave_derive::Columns;

pub struct OrderModel {
    pub delivery_date: i64,
}

// `rename_all` would rename every column in a way the derive does not model.
#[derive(sqlx::FromRow, Columns)]
#[columns(model = OrderModel)]
#[sqlx(rename_all = "camelCase")]
pub struct Order {
    pub delivery_date: i64,
}

fn main() {}
