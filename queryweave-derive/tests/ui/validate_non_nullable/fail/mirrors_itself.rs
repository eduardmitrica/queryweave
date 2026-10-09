use queryweave_derive::ValidateNonNullable;
use validator::Validate;

// A struct mirroring itself never reaches a merge patch model; the error
// points at the path.
#[derive(ValidateNonNullable, Validate)]
#[validate_non_nullable(mirrors = Self, validator)]
pub struct MergePatchOrderDto {
    pub note: queryweave::Patch<String>,
}

fn main() {}
