use queryweave_derive::Filters;

#[derive(Filters)]
struct UserModel {
    #[filter(bool)]
    full_name: String,
}

fn main() {}
