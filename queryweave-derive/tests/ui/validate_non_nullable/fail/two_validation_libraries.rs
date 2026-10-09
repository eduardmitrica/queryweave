use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};

#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub note: queryweave::Patch<String>,
}

// Each validation library defines validate_non_nullable: one at a time.
#[derive(ValidateNonNullable)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator, garde)]
pub struct MergePatchOrderDto {
    pub note: queryweave::Patch<String>,
}

fn main() {}
