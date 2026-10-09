use queryweave_derive::MergePatchNullableFields;

// Merge patches express "absent, null or a value" with `Patch<T>` only: an
// `Option<Option<T>>` is rejected at its type, naming `Patch<T>`.
#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub note: Option<Option<String>>,
}

fn main() {}
