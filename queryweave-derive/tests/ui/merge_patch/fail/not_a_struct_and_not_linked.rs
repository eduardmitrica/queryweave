use queryweave_derive::MergePatch;

// Reading the struct attributes and parsing the input fail independently, so
// both are reported: the item is not a struct, and no merge patch model is
// named.
#[derive(MergePatch)]
enum OrderModel {
    Client(Option<String>),
}

fn main() {}
