use queryweave_derive::Filters;

#[derive(Filters)]
struct UserModel {
    #[sortable]
    age: i32,
}

fn main() {}
