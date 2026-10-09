use queryweave_derive::Filters;

// A second mirrored model, in the same attribute or another one, is rejected
// rather than silently ignored; the error points at the second path.
#[derive(Filters)]
#[filters(mirrors = UserModel, mirrors = OtherModel)]
struct UserDto {
    #[filter(sw)]
    full_name: String,
}

fn main() {}
