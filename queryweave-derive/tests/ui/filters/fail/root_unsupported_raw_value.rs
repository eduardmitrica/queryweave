use queryweave_derive::Filters;

// No Postgres integer holds the whole range of a `u64`.
#[derive(Filters)]
struct UserModel {
    #[filter(lt)]
    visits: u64,
}

fn main() {}
