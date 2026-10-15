// What the code `#[derive(Columns)]` generates for merge-patch writes goes
// through: kept with the `merge-patch` feature, dropped without it (so the
// function below does not exist).
queryweave_sql::__private::if_merge_patch! {
    fn kept() {}
}

fn main() {
    kept();
}
