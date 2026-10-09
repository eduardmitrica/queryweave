use queryweave_derive::Filters;

// Integers without a Postgres type of their own are filterable: they widen to
// the smallest Postgres integer holding their whole range.
#[derive(Filters)]
struct UserModel {
    #[filter(lt, eq, gt)]
    age: u8,

    #[filter(eq)]
    level: i8,

    #[filter(gt)]
    logins: u16,

    #[filter(lt)]
    visits: Option<u32>,
}

fn main() {}
