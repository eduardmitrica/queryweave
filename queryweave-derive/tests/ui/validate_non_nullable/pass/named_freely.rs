use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};
use validator::Validate;

#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub value: Option<f64>,
}

// The DTO links to the model it names, as written: its own name does not have
// to follow the model's.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
pub struct MergePatchShipmentDto {
    pub value: queryweave::Patch<f64>,
}

fn main() {}
