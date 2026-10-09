//! Tests for the output of the [`queryweave_derive::ValidateNonNullable`] proc macro.

use chrono::{DateTime, Utc};
use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrderValue<T = f64>(T);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct OrderDeliveryDate<T = DateTime<Utc>>(T);

#[allow(dead_code)]
#[derive(MergePatchNullableFields)]
struct MergePatchOrderModel {
    value: Option<OrderValue<f64>>,
    delivery_date: Option<OrderDeliveryDate<DateTime<Utc>>>,
}

#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
struct MergePatchOrderDto {
    value: queryweave::Patch<f64>,
    delivery_date: queryweave::Patch<DateTime<Utc>>,
}

#[test]
fn test_validation_succeeds_when_all_preconditions_are_met() {
    let merge_patch = MergePatchOrderDto {
        value: queryweave::Patch::Missing,
        delivery_date: queryweave::Patch::Value(Utc::now()),
    };

    let validation_result = merge_patch.validate_non_nullable();

    assert!(validation_result.is_ok());
}

#[allow(dead_code)]
#[derive(MergePatchNullableFields)]
struct MergePatchInvoiceModel {
    number: queryweave::Patch<String>,
    capacity: Option<i32>,
}

// Fields named differently on the domain merge patch model map onto it with
// `#[equivalent_field(name)]`.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchInvoiceModel, validator)]
struct MergePatchInvoiceDto {
    #[equivalent_field(number)]
    invoice_number: queryweave::Patch<String>,
    #[equivalent_field(capacity)]
    invoice_capacity: queryweave::Patch<i32>,
}

// A renamed field's nullability is looked up under its domain name. Looked up
// under the DTO's name, `invoice_number` would not be found and its `null`
// would be rejected.
#[test]
fn test_a_renamed_nullable_field_may_be_set_to_null() {
    let merge_patch = MergePatchInvoiceDto {
        invoice_number: queryweave::Patch::Null,
        invoice_capacity: queryweave::Patch::Missing,
    };

    assert!(merge_patch.validate_non_nullable().is_ok());
}

// A renamed non-nullable field is still rejected, reported under the DTO's
// name (the one the client sent).
#[test]
fn test_a_renamed_non_nullable_field_is_reported_under_the_dto_name() {
    let merge_patch = MergePatchInvoiceDto {
        invoice_number: queryweave::Patch::Missing,
        invoice_capacity: queryweave::Patch::Null,
    };

    let errors = merge_patch.validate_non_nullable().unwrap_err();

    assert!(errors.field_errors().contains_key("invoice_capacity"));
    assert!(!errors.field_errors().contains_key("capacity"));
}

#[test]
fn test_validation_fails_when_trying_to_set_non_nullable_field_to_null() {
    let merge_patch = MergePatchOrderDto {
        value: queryweave::Patch::Null,
        delivery_date: queryweave::Patch::Value(Utc::now()),
    };

    let validation_result = merge_patch.validate_non_nullable();

    assert!(validation_result.is_err());
}

// A mirror of the DTO (e.g. a gRPC message), itself a mirror: its names map
// onto the DTO's, which map onto the model's.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchInvoiceDto, validator)]
struct MergePatchInvoiceMessage {
    #[equivalent_field(invoice_number)]
    number_text: queryweave::Patch<String>,
    #[equivalent_field(invoice_capacity)]
    capacity_count: queryweave::Patch<i32>,
}

// Nullability follows the chain down to the model, and errors keep the
// mirror's own names.
#[test]
fn test_a_mirror_of_a_mirror_follows_the_chain_to_the_model() {
    let nullable = MergePatchInvoiceMessage {
        number_text: queryweave::Patch::Null,
        capacity_count: queryweave::Patch::Missing,
    };
    assert!(nullable.validate_non_nullable().is_ok());

    let non_nullable = MergePatchInvoiceMessage {
        number_text: queryweave::Patch::Missing,
        capacity_count: queryweave::Patch::Null,
    };
    let errors = non_nullable.validate_non_nullable().unwrap_err();
    assert!(errors.field_errors().contains_key("capacity_count"));
}

// Each link answers under its own names, and the chain's root is the model.
#[test]
fn test_every_link_answers_under_its_own_names() {
    use queryweave::NullableFields;

    fn root_of<T: NullableFields<Root = MergePatchInvoiceModel>>() {}
    root_of::<MergePatchInvoiceModel>();
    root_of::<MergePatchInvoiceDto>();
    root_of::<MergePatchInvoiceMessage>();

    assert!(MergePatchInvoiceModel::is_nullable("number"));
    assert!(MergePatchInvoiceDto::is_nullable("invoice_number"));
    assert!(MergePatchInvoiceMessage::is_nullable("number_text"));
    assert!(!MergePatchInvoiceMessage::is_nullable("capacity_count"));
    // Names of another link are not this one's.
    assert!(!MergePatchInvoiceMessage::is_nullable("invoice_number"));
}
