use queryweave_derive::Filters;

#[derive(Filters)]
pub struct UserModel {
    #[filter(sw)]
    #[sortable]
    pub full_name: String,
}

// A validation error carries the rejected value, serialized: `validator`
// needs `serde`, and the error points at `validator`.
#[derive(Filters)]
#[filters(mirrors = UserModel, utoipa, validator)]
struct UserDto {
    #[filter(sw)]
    #[sortable]
    full_name: String,
}

fn main() {}
