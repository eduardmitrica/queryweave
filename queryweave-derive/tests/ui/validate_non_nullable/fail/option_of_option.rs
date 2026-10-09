use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};
use validator::Validate;

#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub note: queryweave::Patch<String>,
}

// The DTO's `null` would never be seen in an `Option<Option<T>>`: rejected at
// its type, naming `Patch<T>`.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
pub struct MergePatchOrderDto {
    pub note: Option<Option<String>>,
}

fn main() {}
