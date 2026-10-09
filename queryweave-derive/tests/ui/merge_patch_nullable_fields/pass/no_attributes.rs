use queryweave::MergePatchNullableFields;
use queryweave_derive::MergePatchNullableFields;

#[derive(MergePatchNullableFields)]
struct MergePatchOrderModel {
    client: queryweave::Patch<String>,
}

fn main() {
    let _ = MergePatchOrderModel::is_nullable("client");
}
