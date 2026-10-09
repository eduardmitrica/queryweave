use queryweave_derive::Filters;

// An unknown `#[queryweave(...)]` option is rejected rather than silently
// ignored (here, a misspelt `crate`).
#[derive(Filters)]
#[queryweave(krate = my::runtime)]
struct OrderModel {
    #[filter(eq)]
    value: i32,
}

fn main() {}
