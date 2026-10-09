use queryweave_derive::Columns;

pub struct OrderModel {
    pub number: String,
}

impl queryweave::PersistedFields for OrderModel {
    const FIELD_NAMES: &'static [&'static str] = &["number"];
}

pub struct UserModel {
    pub username: String,
}

impl queryweave::PersistedFields for UserModel {
    const FIELD_NAMES: &'static [&'static str] = &["username"];
}

#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    pub number: String,
}

// The data carries its model, and it is that model's fields that are checked
// against the entity: `UserModel` data cannot be written through `Order`,
// which has no column for `username`.
fn main() {
    let mut query_builder = sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE orders SET ");
    let user_values = queryweave::__private::model_data::<UserModel, _>(Vec::new());
    let _ = Order::push_assignments(&mut query_builder, user_values);
}
