use queryweave_derive::{MergePatch, MergePatchNullableFields, RawValue};

#[derive(PartialEq, RawValue)]
struct Capacity<T = i32>(T);

#[derive(MergePatchNullableFields)]
struct MergePatchOrderModel {
    capacity: Option<String>,
}

// `String` does not convert `Into<Capacity<i32>>`.
#[derive(MergePatch)]
#[merge_patch(patch = MergePatchOrderModel)]
struct OrderModel {
    capacity: Capacity<i32>,
}

fn main() {}
