use queryweave_derive::ValidateNonNullable;
use chrono::{DateTime, Utc};
use validator::Validate;

// The domain merge patch model must be named `MergePatch…`; the error points
// at the path naming it.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = domain::models::PatchOrderModel, validator)]
pub struct MergePatchOrderDto {
    pub value: queryweave::Patch<f64>,
    pub delivery_date: queryweave::Patch<DateTime<Utc>>,
}

fn main() {}
