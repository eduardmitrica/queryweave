use queryweave_derive::Filters;

#[derive(Filters)]
pub struct UserModel {
    #[filter(sw)]
    pub full_name: String,
}

// An integration is set by being written: it takes no value.
#[derive(Filters)]
#[filters(mirrors = UserModel, serde = true)]
struct UserDto {
    #[filter(sw)]
    full_name: String,
}

fn main() {}
