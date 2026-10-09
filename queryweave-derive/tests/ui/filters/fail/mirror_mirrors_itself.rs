use queryweave_derive::Filters;

// A struct mirroring itself never reaches a root model; the error points at
// the path (here its own name).
#[derive(Filters)]
#[filters(mirrors = UserDto)]
pub struct UserDto {
    #[filter(sw)]
    pub full_name: String,
}

fn main() {}
