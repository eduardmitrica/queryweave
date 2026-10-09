//! Tests for the output of the [`queryweave_derive::RawValue`] proc macro.

use pretty_assertions::assert_eq;
use queryweave::RawValue as _;
use queryweave_derive::RawValue;

#[derive(Debug, Clone, PartialEq, RawValue)]
struct Username<T = String>(T);

#[derive(Debug, Clone, Copy, PartialEq, RawValue)]
struct OrderId(i32);

/// A value object wrapping another value object.
#[derive(Debug, Clone, PartialEq, RawValue)]
struct Handle<T = Username>(T);

#[test]
fn test_a_generic_value_object_has_its_inner_raw_value() {
    let raw: String = Username(String::from("jane")).to_raw();

    assert_eq!(raw, "jane");
}

#[test]
fn test_a_concrete_value_object_has_its_inner_raw_value() {
    let raw: i32 = OrderId(7).to_raw();

    assert_eq!(raw, 7);
}

#[test]
fn test_nested_value_objects_resolve_to_the_innermost_raw_value() {
    let raw: String = Handle(Username(String::from("jane"))).to_raw();

    assert_eq!(raw, "jane");
}

#[test]
fn test_an_optional_value_object_has_an_optional_raw_value() {
    let raw: Option<String> = Some(Username(String::from("jane"))).to_raw();

    assert_eq!(raw, Some(String::from("jane")));
}
