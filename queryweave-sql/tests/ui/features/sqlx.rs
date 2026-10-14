// What writing a model's data into sqlx's `QueryBuilder` needs: builds with
// the `sqlx` feature, and names the feature to enable without it.
const fn requires_target<Q: queryweave_sql::ConditionsTarget>() {}

const _: () = requires_target::<sqlx::QueryBuilder<'static, sqlx::Postgres>>();

fn main() {}
