use queryweave_derive::Filters;

/// A value object that does not derive `RawValue`.
struct Tag(String);

#[derive(Filters)]
struct UserModel {
    #[filter(eq)]
    tag: Tag,
}

fn main() {}
