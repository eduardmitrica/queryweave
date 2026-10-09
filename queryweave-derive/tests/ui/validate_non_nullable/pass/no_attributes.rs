use chrono::{DateTime, Utc};
use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderValue<T = f64>(T);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct OrderDeliveryDate<T = DateTime<Utc>>(T);

#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub value: Option<OrderValue<f64>>,
    pub delivery_date: Option<OrderDeliveryDate<DateTime<Utc>>>,
}

#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
pub struct MergePatchOrderDto {
    pub value: queryweave::Patch<f64>,
    pub delivery_date: queryweave::Patch<DateTime<Utc>>,
}

fn main() {}
