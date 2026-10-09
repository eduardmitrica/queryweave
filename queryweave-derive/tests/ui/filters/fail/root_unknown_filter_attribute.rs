use queryweave_derive::Filters;

#[derive(Filters)]
struct ObjectModel {
    #[filter(boolean)]
    was_deleted: bool,
}

fn main() {}
