use queryweave::MergePatchNullableFields;
use queryweave_derive::MergePatchNullableFields;

struct OrderId(i32);

// An `Option<T>` field cannot be cleared and needs no annotation; a field that
// is not patched at all (an id) is marked `#[merge_patch(skip)]`.
#[derive(MergePatchNullableFields)]
struct MergePatchOrderModel {
    #[merge_patch(skip)]
    id: OrderId,
    client: queryweave::Patch<String>,
    capacity: Option<i32>,
}

fn main() {
    let _ = MergePatchOrderModel::is_nullable("capacity");
}
