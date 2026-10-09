use queryweave_derive::Filters;

#[derive(Filters)]
struct UserModel {
    #[filter(sw)]
    full_name: String,
}

fn main() {}
