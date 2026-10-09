use queryweave_derive::Filters;

// The mirrored model must derive `Filters`: the mirror names the model's generated
// types through `queryweave::Filterable`. The error points at the path naming
// the model.
#[allow(dead_code)]
pub struct UserModel {
    full_name: String,
}

#[derive(Filters)]
#[filters(mirrors = UserModel)]
struct UserDto {
    #[filter(sw)]
    full_name: String,
}

fn main() {}
