//! Without a flag, only the library-free `rejected_nulls`; with `garde`, a
//! `validate_non_nullable` returning garde's report.
use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};

#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub note: queryweave::Patch<String>,
    pub capacity: Option<i32>,
}

#[derive(ValidateNonNullable)]
#[validate_non_nullable(mirrors = MergePatchOrderModel)]
pub struct MergePatchOrderDto {
    pub note: queryweave::Patch<String>,
    pub capacity: queryweave::Patch<i32>,
}

mod with_garde {
    use queryweave_derive::ValidateNonNullable;

    #[derive(garde::Validate, ValidateNonNullable)]
    #[validate_non_nullable(mirrors = super::MergePatchOrderModel, garde)]
    pub struct MergePatchOrderDto {
        #[garde(skip)]
        pub note: queryweave::Patch<String>,
        #[garde(skip)]
        pub capacity: queryweave::Patch<i32>,
    }
}

fn main() {
    let plain = MergePatchOrderDto {
        note: queryweave::Patch::Null,
        capacity: queryweave::Patch::Null,
    };
    let _: Vec<&'static str> = plain.rejected_nulls();

    let with_garde = with_garde::MergePatchOrderDto {
        note: queryweave::Patch::Missing,
        capacity: queryweave::Patch::Null,
    };
    let _: Result<(), garde::Report> = with_garde.validate_non_nullable();
}
