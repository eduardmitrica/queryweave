//! Tests for the output of the [`queryweave_derive::MergePatch`] proc macro.

use pretty_assertions::assert_eq;
use queryweave_derive::{MergePatch, MergePatchNullableFields, RawValue};

/// A value object the patch carries where the model keeps the raw value, like
/// `OrderDeliveryDate` for `OrderModel::delivery_date`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Deadline<T = i64>(T);

impl From<Deadline<Self>> for i64 {
    fn from(value: Deadline<Self>) -> Self {
        value.0
    }
}

/// A non-`Copy` value object, patched as-is. `RawValue` is how the generated
/// `append_to_query` reaches the inner value to bind.
#[derive(Debug, Clone, PartialEq, RawValue)]
struct Label<T = String>(T);

// `created_by_user_with_id` is deliberately never read: the model skips it.
#[allow(dead_code)]
#[derive(Default, MergePatchNullableFields)]
struct MergePatchOrderModel {
    client: queryweave::Patch<String>,
    label: Option<Label<String>>,
    capacity: Option<i32>,
    #[allow(dead_code)]
    deadline: Option<Deadline<i64>>,
    #[merge_patch(skip)]
    created_by_user_with_id: i32,
}

#[derive(Debug, Clone, PartialEq, MergePatch)]
#[merge_patch(patch = MergePatchOrderModel)]
struct OrderModel {
    client: Option<String>,
    label: Label<String>,
    capacity: i32,
    deadline: i64,

    #[merge_patch(skip)]
    created_by_user_with_id: i32,
}

fn order_model() -> OrderModel {
    OrderModel {
        client: Some(String::from("Client")),
        label: Label(String::from("Label")),
        capacity: 10,
        deadline: 100,
        created_by_user_with_id: 1,
    }
}

fn merged(merge_patch: MergePatchOrderModel) -> OrderModel {
    let mut model = order_model();
    model.merge(merge_patch);
    model
}

// A patch with every field absent leaves the model untouched.
#[test]
fn test_an_empty_patch_changes_nothing() {
    assert_eq!(merged(MergePatchOrderModel::default()), order_model());
}

// `Patch::Value` sets a nullable field, `Patch::Null` clears it.
#[test]
fn test_a_nullable_field_can_be_set_and_cleared() {
    let set = merged(MergePatchOrderModel {
        client: queryweave::Patch::Value(String::from("Other")),
        ..Default::default()
    });
    assert_eq!(set.client, Some(String::from("Other")));

    let cleared = merged(MergePatchOrderModel {
        client: queryweave::Patch::Null,
        ..Default::default()
    });
    assert_eq!(cleared.client, None);
}

// Present fields replace the model's, both `Copy` and non-`Copy` ones, and a
// value object is converted into the model's raw field type through `Into`.
#[test]
fn test_present_fields_replace_the_model_fields() {
    let model = merged(MergePatchOrderModel {
        label: Some(Label(String::from("Other"))),
        capacity: Some(20),
        deadline: Some(Deadline(200)),
        ..Default::default()
    });

    assert_eq!(
        model,
        OrderModel {
            label: Label(String::from("Other")),
            capacity: 20,
            deadline: 200,
            ..order_model()
        }
    );
}

// A `#[merge_patch(skip)]` field is never patched, even though the patch has a
// field of the same name.
#[test]
fn test_a_skipped_field_is_not_patched() {
    let model = merged(MergePatchOrderModel {
        created_by_user_with_id: 2,
        ..Default::default()
    });

    assert_eq!(model.created_by_user_with_id, 1);
}

// The fields a merge patch may write, in declaration order, without the
// skipped one: what the persistence layer must have columns for.
#[test]
fn test_patchable_field_names_list_the_fields_not_skipped() {
    assert_eq!(
        <OrderModel as queryweave::PersistedFields>::FIELD_NAMES,
        &["client", "label", "capacity", "deadline"]
    );
}
