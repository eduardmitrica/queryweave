use queryweave_derive::Filters;

#[derive(Filters)]
struct UserModel {
    #[filter(lt)]
    full_name: String,
}

fn main() {}
