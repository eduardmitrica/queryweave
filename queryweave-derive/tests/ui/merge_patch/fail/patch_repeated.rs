use queryweave_derive::MergePatch;

pub struct MergePatchOrderModel {
    pub client: Option<String>,
}

// A second merge patch model is rejected rather than silently ignored.
#[derive(MergePatch)]
#[merge_patch(patch = MergePatchOrderModel)]
#[merge_patch(patch = MergePatchOrderModel)]
struct OrderModel {
    client: Option<String>,
}

fn main() {}
