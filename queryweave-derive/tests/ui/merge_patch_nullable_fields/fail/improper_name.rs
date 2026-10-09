use queryweave_derive::MergePatchNullableFields;

#[derive(MergePatchNullableFields)]
struct OrderModel {
    client: queryweave::Patch<String>,
}

fn main() {}
