use queryweave_derive::MergePatch;

#[derive(MergePatch)]
struct OrderModel {
    client: Option<String>,
}

fn main() {}
