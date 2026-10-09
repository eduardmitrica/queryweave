use queryweave_derive::Filters;

// Two generated types cannot share a name; the error points at the second.
#[derive(Filters)]
#[filters(sort_field = UserSort, sort_details = UserSort)]
pub struct UserModel {
    #[filter(sw)]
    #[sortable]
    pub username: String,
}

fn main() {}
