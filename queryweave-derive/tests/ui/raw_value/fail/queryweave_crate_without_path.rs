use queryweave_derive::RawValue;

// `crate` names where the runtime crate lives with a path.
#[derive(RawValue)]
#[queryweave(crate)]
struct Label<T = String>(T);

fn main() {}
