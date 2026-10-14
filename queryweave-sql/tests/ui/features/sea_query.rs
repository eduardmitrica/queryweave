// What writing a model's data into a sea-query statement needs: builds with
// the `sea-query` feature, and names the feature to enable without it.
const fn requires_target<Q: queryweave_sql::ConditionsTarget>() {}

const _: () = requires_target::<sea_query::SelectStatement>();

fn main() {}
