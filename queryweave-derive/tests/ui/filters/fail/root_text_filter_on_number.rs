use queryweave_derive::Filters;

#[derive(Filters)]
struct UserModel {
    #[filter(sw)]
    age: i32,
}

fn main() {}
