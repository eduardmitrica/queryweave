use queryweave_derive::MergePatch;

pub struct PatchOrderModel {
    pub client: Option<String>,
}

// Mapping the merge patch model and analyzing the fields fail independently,
// so both are reported: the model's name does not start with `MergePatch`,
// and `#[merge_patch(...)]` only takes `skip`.
#[derive(MergePatch)]
#[merge_patch(patch = PatchOrderModel)]
struct OrderModel {
    #[merge_patch(ignore)]
    client: Option<String>,
}

fn main() {}
