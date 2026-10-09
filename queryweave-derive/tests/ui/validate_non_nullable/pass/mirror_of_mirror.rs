use queryweave_derive::{MergePatchNullableFields, ValidateNonNullable};
use validator::Validate;

#[derive(MergePatchNullableFields)]
pub struct MergePatchOrderModel {
    pub note: queryweave::Patch<String>,
}

#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
pub struct MergePatchOrderDto {
    pub note: queryweave::Patch<String>,
}

mod grpc {
    use queryweave_derive::ValidateNonNullable;
    use validator::Validate;

    // A mirror of the DTO, itself a mirror: its names map onto the DTO's.
    #[derive(ValidateNonNullable, Validate)]
    #[validate_non_nullable(mirrors = super::MergePatchOrderDto, validator)]
    pub struct MergePatchOrderMessage {
        #[equivalent_field(note)]
        pub remark: queryweave::Patch<String>,
    }
}

fn main() {
    let message = grpc::MergePatchOrderMessage { remark: queryweave::Patch::Null };
    let _ = message.validate_non_nullable();
}
