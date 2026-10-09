use queryweave_derive::{MergePatch, MergePatchNullableFields};

#[derive(MergePatchNullableFields)]
struct MergePatchOrderModel {
    client: queryweave::Patch<String>,
}

#[derive(MergePatch)]
#[merge_patch(patch = MergePatchOrderModel)]
struct OrderModel {
    client: Option<String>,

    #[merge_patch(skip)]
    created_by_user_with_id: u32,
}

fn main() {}
