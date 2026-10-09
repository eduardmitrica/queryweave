use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};
use validator::Validate;

#[derive(MergePatchNullableFields)]
pub struct MergePatchInvoiceModel {
    pub number: queryweave::Patch<String>,
}

// `numbr` is not a field of `MergePatchInvoiceModel`, so the nullability of
// `invoice_number` could not be looked up; reported at the name.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchInvoiceModel, validator)]
pub struct MergePatchInvoiceDto {
    #[equivalent_field(numbr)]
    pub invoice_number: queryweave::Patch<String>,
}

fn main() {}
