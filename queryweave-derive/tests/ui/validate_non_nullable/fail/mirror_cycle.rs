// Every chain of mirrors must end at a merge patch model: two structs
// mirroring each other have none, so their `NullableFields::Root` cannot be
// resolved (rather than asking each other forever at runtime).
mod api {
    use queryweave_derive::ValidateNonNullable;
    use validator::Validate;

    #[derive(ValidateNonNullable, Validate)]
    #[validate_non_nullable(mirrors = super::grpc::MergePatchOrderMessage, validator)]
    pub struct MergePatchOrderDto {
        pub note: queryweave::Patch<String>,
    }
}

mod grpc {
    use queryweave_derive::ValidateNonNullable;
    use validator::Validate;

    #[derive(ValidateNonNullable, Validate)]
    #[validate_non_nullable(mirrors = super::api::MergePatchOrderDto, validator)]
    pub struct MergePatchOrderMessage {
        pub note: queryweave::Patch<String>,
    }
}

fn main() {}
