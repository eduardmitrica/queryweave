use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};

#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub note: queryweave::Patch<String>,
}

// `serde` is no validation library: it does not define validate_non_nullable.
#[derive(ValidateNonNullable)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, serde)]
pub struct MergePatchOrderDto {
    pub note: queryweave::Patch<String>,
}

fn main() {}
