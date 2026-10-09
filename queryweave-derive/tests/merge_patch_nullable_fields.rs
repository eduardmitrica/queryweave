//! Tests for the output of the [`queryweave_derive::MergePatchNullableFields`] proc macro.

use queryweave::MergePatchNullableFields;
use queryweave_derive::MergePatchNullableFields;

#[derive(Debug, Clone)]
struct UserCode<T = String>(T);

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct UserId(u32);

#[allow(dead_code)]
#[derive(MergePatchNullableFields)]
struct MergePatchUserModel {
    #[merge_patch(skip)]
    id: UserId,
    code: queryweave::Patch<UserCode<String>>,
    username: queryweave::Patch<String>,
    secondary_code: Option<String>,
}

#[allow(dead_code)]
#[derive(MergePatchNullableFields)]
struct MergePatchOrderModel {
    #[merge_patch(skip)]
    id: UserId,
    client: Option<String>,
}

// Exactly the `queryweave::Patch<T>` fields may be set to `null`; `Option`
// fields, skipped fields and fields the struct does not have may not.
#[test]
fn test_only_patch_fields_are_nullable() {
    assert!(MergePatchUserModel::is_nullable("code"));
    assert!(MergePatchUserModel::is_nullable("username"));
    assert!(!MergePatchUserModel::is_nullable("secondary_code"));
    assert!(!MergePatchUserModel::is_nullable("id"));
    assert!(!MergePatchUserModel::is_nullable("unknown"));
}

#[test]
fn test_no_field_is_nullable_without_patch_fields() {
    assert!(!MergePatchOrderModel::is_nullable("client"));
}
