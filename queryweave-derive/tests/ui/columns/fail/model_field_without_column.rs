use queryweave_derive::Columns;

pub struct OrderModel {
    pub number: String,
    pub client: String,
}

// The domain fields a merge patch of `OrderModel` may write.
impl queryweave::PersistedFields for OrderModel {
    const FIELD_NAMES: &'static [&'static str] = &["number", "client"];
}

// `client` has no column on `Order`, so writing `OrderModel` data through it
// must not build.
#[derive(Columns)]
#[columns(model = OrderModel)]
pub struct Order {
    pub number: String,
}

fn main() {
    let mut query_builder = sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE orders SET ");
    let changed_values = queryweave::__private::model_data::<OrderModel, _>(Vec::new());
    let _ = Order::push_assignments(&mut query_builder, changed_values);
}
