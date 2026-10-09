//! The derives under the custom conventions of
//! `queryweave-custom-conventions`: every rule reaches the generated code.

use pretty_assertions::assert_eq;
use queryweave_custom_conventions::{
    Filters, MergePatch, MergePatchNullableFields, ValidateNonNullable,
};
use validator::Validate;

/// `queryweave`, re-exported where the conventions say it is.
mod runtime {
    pub use queryweave::*;
}

/// `queryweave_api`, re-exported where the conventions say it is.
mod api_runtime {
    pub use queryweave_api::*;
}

/// A root model: its single-segment name is fine under these conventions.
#[derive(Debug, Clone, PartialEq, Filters, MergePatch)]
#[merge_patch(patch = CustomerPatch)]
struct Customer {
    #[filter(sw)]
    #[sortable]
    name: String,

    #[filter(bool)]
    active: bool,

    note: Option<String>,
}

#[derive(Default, MergePatchNullableFields)]
struct CustomerPatch {
    name: Option<String>,
    active: Option<bool>,
    note: queryweave::Patch<String>,
}

/// A mirror of [`Customer`], as an API DTO would be.
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Filters)]
#[filters(mirrors = Customer, serde, utoipa, validator)]
struct CustomerDto {
    #[filter(sw)]
    #[sortable]
    name: String,

    #[filter(bool)]
    active: bool,
}

#[derive(Validate, ValidateNonNullable)]
#[validate_non_nullable(mirrors = CustomerPatch, validator)]
struct CustomerPatchDto {
    name: queryweave::Patch<String>,
    note: queryweave::Patch<String>,
}

// The generated types are named by the conventions, the filter fields and
// query parameters put the operator first, and the mirror converts into the
// root model's types through the re-exported runtime.
#[test]
fn test_filters_follow_the_conventions() {
    let query_params: CustomerDtoSearchParams = serde_json::from_value(serde_json::json!({
        "sw_name": "Ad",
        "active": true,
        "sort": [{ "field": "name", "order": "descending" }],
    }))
    .unwrap();

    let query: CustomerSearch = query_params.into();
    let filters: &CustomerFilters = &query.and;

    assert_eq!(filters.sw_name, Some(String::from("Ad")));
    assert_eq!(filters.active, Some(true));
    let sort: Vec<CustomerSort> = query.sort.unwrap();
    assert_eq!(sort.len(), 1);
    assert_eq!(sort[0].field, CustomerSortKey::Name);
    assert_eq!(sort[0].order, runtime::SortDirectionModel::Descending);
    let page: Option<CustomerDtoPage> = None;
    assert!(page.is_none());
}

#[test]
fn test_merge_patch_follows_the_conventions() {
    let mut customer = Customer {
        name: String::from("Ada"),
        active: true,
        note: Some(String::from("first")),
    };

    customer.merge(CustomerPatch {
        name: Some(String::from("Grace")),
        note: queryweave::Patch::Null,
        ..CustomerPatch::default()
    });

    assert_eq!(
        customer,
        Customer {
            name: String::from("Grace"),
            active: true,
            note: None,
        }
    );
}

#[test]
fn test_validate_non_nullable_follows_the_conventions() {
    let nulled_name = CustomerPatchDto {
        name: queryweave::Patch::Null,
        note: queryweave::Patch::Missing,
    };
    let nulled_note = CustomerPatchDto {
        name: queryweave::Patch::Missing,
        note: queryweave::Patch::Null,
    };

    assert!(nulled_name.validate_non_nullable().is_err());
    assert!(nulled_note.validate_non_nullable().is_ok());
}

/// The types the `wire` integration names: a toy serializer other than serde.
mod wire {
    /// Encoding into the toy wire format.
    pub trait Encode {
        fn encode(&self) -> String;
    }

    /// The wire's own sort direction, converting both ways with the core one.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Direction {
        Up,
        Down,
    }

    impl Encode for Direction {
        fn encode(&self) -> String {
            String::from(match self {
                Self::Up => "up",
                Self::Down => "down",
            })
        }
    }

    impl From<Direction> for crate::runtime::SortDirectionModel {
        fn from(value: Direction) -> Self {
            match value {
                Direction::Up => Self::Ascending,
                Direction::Down => Self::Descending,
            }
        }
    }

    impl From<crate::runtime::SortDirectionModel> for Direction {
        fn from(value: crate::runtime::SortDirectionModel) -> Self {
            match value {
                crate::runtime::SortDirectionModel::Ascending => Self::Up,
                crate::runtime::SortDirectionModel::Descending => Self::Down,
            }
        }
    }

    /// The wire's own page.
    pub struct Page<T> {
        pub items: Vec<T>,
    }
}

/// A mirror of [`Customer`] on the wire, with neither serde nor utoipa.
#[allow(dead_code)]
#[derive(Debug, Filters)]
#[filters(mirrors = Customer, wire)]
struct CustomerMessage {
    #[filter(sw)]
    #[sortable]
    name: String,

    #[filter(bool)]
    active: bool,
}

// An integration of the conventions' own is opted into like a built-in one:
// its sort direction, page type, derives and items reach the generated code.
#[test]
fn test_an_integration_of_the_conventions_reaches_the_generated_code() {
    use wire::Encode;

    let sort = CustomerMessageSort {
        field: CustomerMessageSortKey::Name,
        order: wire::Direction::Down,
    };
    assert_eq!(sort.encode(), "Name:down");
    assert!(CustomerMessageSortKey::Name <= CustomerMessageSortKey::Name);

    let query_params = CustomerMessageSearchParams {
        page: Some(2),
        sw_name: Some(String::from("Ad")),
        sort: Some(vec![sort]),
        ..Default::default()
    };
    assert_eq!(query_params.encode(), "page=Some(2);page_size=None");

    // The wire's direction converts into the root model's.
    let query: CustomerSearch = query_params.into();
    assert_eq!(
        query.sort.unwrap()[0].order,
        runtime::SortDirectionModel::Descending
    );

    let page: CustomerMessagePage = wire::Page { items: Vec::new() };
    assert!(page.items.is_empty());
}
