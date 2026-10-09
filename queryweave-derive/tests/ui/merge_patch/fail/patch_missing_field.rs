use queryweave_derive::{MergePatch, MergePatchNullableFields};

#[derive(MergePatchNullableFields)]
struct MergePatchOrderModel {
    client: queryweave::Patch<String>,
}

#[derive(MergePatch)]
#[merge_patch(patch = MergePatchOrderModel)]
struct OrderModel {
    client: Option<String>,
    capacity: i32,
}

fn main() {}
