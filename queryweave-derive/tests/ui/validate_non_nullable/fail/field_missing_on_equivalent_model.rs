use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};
use validator::Validate;

#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub value: Option<f64>,
}

// `amount` has no counterpart on the domain merge patch model, so its
// nullability could not be looked up.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
pub struct MergePatchOrderDto {
    pub value: queryweave::Patch<f64>,
    pub amount: queryweave::Patch<f64>,
}

fn main() {}
