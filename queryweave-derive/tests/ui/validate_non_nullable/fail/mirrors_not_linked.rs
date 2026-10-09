use queryweave_derive::ValidateNonNullable;
use chrono::{DateTime, Utc};
use validator::Validate;

#[derive(ValidateNonNullable, Validate)]
pub struct MergePatchOrderDto {
    pub value: queryweave::Patch<f64>,
    pub delivery_date: queryweave::Patch<DateTime<Utc>>,
}

fn main() {}
