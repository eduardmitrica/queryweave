use queryweave_derive::ValidateNonNullable;
use chrono::{DateTime, Utc};
use validator::Validate;

// A second domain merge patch model is rejected rather than silently ignored;
// the error points at the second path.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
pub struct MergePatchOrderDto {
    pub value: queryweave::Patch<f64>,
    pub delivery_date: queryweave::Patch<DateTime<Utc>>,
}

fn main() {}
