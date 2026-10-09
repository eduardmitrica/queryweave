use queryweave_derive::Filters;

// `mirrors` names the mirrored root model with a path.
#[derive(Filters)]
#[filters(mirrors)]
struct UserDto {
    #[filter(sw)]
    full_name: String,
}

fn main() {}
