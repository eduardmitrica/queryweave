use queryweave_derive::Filters;

#[derive(Filters)]
struct User {
    #[filter(sw)]
    full_name: String,
}

fn main() {}
