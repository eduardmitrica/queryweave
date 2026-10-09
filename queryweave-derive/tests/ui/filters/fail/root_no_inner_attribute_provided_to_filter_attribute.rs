use queryweave_derive::Filters;

#[derive(Filters)]
struct UserModel {
    #[filter]
    email_address: String,

    #[filter()]
    username: String,
}

fn main() {}
