use queryweave_derive::MergePatch;

// The patch struct must be a declared merge patch model (deriving
// `MergePatchNullableFields`, which checks its fields); the error points at
// the path.
#[derive(Default)]
pub struct MergePatchOrderModel {
    pub client: Option<String>,
}

#[derive(MergePatch)]
#[merge_patch(patch = MergePatchOrderModel)]
pub struct OrderModel {
    pub client: String,
}

fn main() {}
