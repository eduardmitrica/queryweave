// What the code `#[derive(Columns)]` generates for a Diesel entity goes
// through: kept with the `diesel` feature, dropped without it (so the
// function below does not exist).
queryweave_sql::__private::if_diesel! {
    fn kept() {}
}

fn main() {
    kept();
}
