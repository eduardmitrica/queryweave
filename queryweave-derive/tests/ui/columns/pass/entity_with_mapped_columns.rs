use queryweave_sql::Columns as _;
use queryweave_derive::Columns;

pub struct OrderModel {
    pub id: i32,
    pub number: String,
    pub delivery_date: i64,
}

#[derive(sqlx::FromRow, Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    pub id: i32,
    #[sqlx(rename = "order_number")]
    pub number: String,
    #[domain_field(delivery_date)]
    pub delivered_at: i64,
    #[sqlx(skip)]
    pub cached: bool,
}

fn main() {
    assert_eq!(Order::NAME, "Order");
    assert_eq!(Order::column("number"), Some("order_number"));
    assert_eq!(Order::column("delivery_date"), Some("delivered_at"));
    assert_eq!(Order::column("cached"), None);
}
