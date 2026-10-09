use queryweave_derive::Filters;

// A root model generates no query parameters, so it cannot rename them; the
// error points at the name.
#[derive(Filters)]
#[filters(query_params = UserSearchParams)]
pub struct UserModel {
    #[filter(sw)]
    pub username: String,
}

fn main() {}
