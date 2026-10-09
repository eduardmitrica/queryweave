use queryweave_derive::Filters;

// An unknown `#[filters(...)]` option is rejected rather than silently
// ignored (here, a misspelt `mirrors`).
#[derive(Filters)]
#[filters(mirror = UserModel)]
struct UserDto {
    #[filter(sw)]
    full_name: String,
}

fn main() {}
