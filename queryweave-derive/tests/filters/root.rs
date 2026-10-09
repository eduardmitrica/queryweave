//! `Filters` on root models.

use queryweave::{
    FilterCondition, FilterOperator, PersistedFields, SortDirectionModel, SortKey, Value,
};
use queryweave_derive::{Filters, RawValue};

/// A type alias: the derive used to reject it, since it could not recognise
/// `Price` as an `f64` by name.
type Price = f64;

/// A value object used without its generic argument: the derive used to
/// require `Sku<String>` to find the inner type.
#[derive(Debug, Clone, PartialEq, RawValue)]
struct Sku<T = String>(T);

// Field types the old name-based classification got wrong; the compiler now
// resolves each through `queryweave::RawValue`.
#[allow(dead_code)]
#[derive(Filters)]
struct ProductModel {
    #[filter(lt, gt)]
    price: Price,

    #[filter(eq, sw)]
    sku: Option<Sku>,

    #[filter(lt)]
    released_on: chrono::NaiveDate,
}

// The generated filter fields hold the raw values, resolved by the compiler.
#[test]
fn test_filter_fields_hold_the_resolved_raw_values() {
    let released_on = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let filters = ProductsFilterOptionsModel {
        price_lt: Some(9.5_f64),
        price_gt: None,
        sku_eq: Some(String::from("SKU-1")),
        sku_sw: None,
        released_on_lt: Some(released_on),
    };

    assert_eq!(
        filters.conditions().into_vec(),
        vec![
            FilterCondition {
                field: "price",
                operator: FilterOperator::LessThan,
                value: Value::F64(Some(9.5)),
            },
            FilterCondition {
                field: "sku",
                operator: FilterOperator::Equal,
                value: Value::Text(Some(String::from("SKU-1"))),
            },
            FilterCondition {
                field: "released_on",
                operator: FilterOperator::LessThan,
                value: Value::NaiveDate(Some(released_on)),
            },
        ]
    );
}

#[allow(dead_code)]
#[derive(Filters)]
struct UserModel {
    #[filter(sw)]
    full_name: String,
}

#[allow(dead_code)]
#[derive(Filters)]
struct OrderModel {
    #[filter(lt, gt, eq)]
    #[sortable]
    order_value: i32,

    #[sortable]
    order_number: String,
}

// Field names containing text that looks like a filter suffix (`_c`, `_sw`,
// `_eq`, ...) in the middle, or as a suffix of their own.
#[allow(dead_code)]
#[derive(Filters)]
struct TicketModel {
    #[filter(bool)]
    is_closed: bool,

    #[filter(eq, gt)]
    user_count: i32,

    #[filter(c)]
    name_c: String,
}

// The generated conditions carry the field name as the column, untouched, with
// the operator taken from the `#[filter(...)]` meta (`bool` is equality).
#[test]
fn test_conditions_carry_the_column_verbatim_with_its_operator() {
    let conditions = TicketsFilterOptionsModel {
        is_closed: Some(true),
        user_count_eq: Some(1),
        user_count_gt: Some(2),
        name_c_c: Some(String::from("x")),
    }
    .conditions();

    assert_eq!(
        conditions
            .as_slice()
            .iter()
            .map(|condition| (condition.field, condition.operator))
            .collect::<Vec<_>>(),
        vec![
            ("is_closed", FilterOperator::Equal),
            ("user_count", FilterOperator::Equal),
            ("user_count", FilterOperator::GreaterThan),
            ("name_c", FilterOperator::Contains),
        ]
    );
}

// The requested sorting comes out as sort keys, in order, each naming the
// domain field (snake cased, as declared) and its direction.
#[test]
fn test_sort_keys_follow_the_requested_sorting() {
    let query = OrdersQueryModel {
        sort: Some(vec![
            OrderSortDetailsModel {
                field: OrderSortFieldModel::OrderNumber,
                order: SortDirectionModel::Descending,
            },
            OrderSortDetailsModel {
                field: OrderSortFieldModel::OrderValue,
                order: SortDirectionModel::Ascending,
            },
        ]),
        ..Default::default()
    };

    assert_eq!(
        query.sort_keys().into_vec(),
        vec![
            SortKey {
                field: "order_number",
                direction: SortDirectionModel::Descending,
            },
            SortKey {
                field: "order_value",
                direction: SortDirectionModel::Ascending,
            },
        ]
    );
    assert!(OrdersQueryModel::default().sort_keys().is_empty());
}

// The fields reaching the persistence layer are published once each, in
// declaration order: the filtered ones on the filter options, plus the
// sortable ones on the query model.
#[test]
fn test_field_names_list_the_filtered_and_sorted_fields_once() {
    // `user_count` carries two filters but is listed once.
    assert_eq!(
        <TicketsFilterOptionsModel as PersistedFields>::FIELD_NAMES,
        &["is_closed", "user_count", "name_c"]
    );
    // `order_value` is filtered and sortable; `order_number` only sortable.
    assert_eq!(
        <OrdersFilterOptionsModel as PersistedFields>::FIELD_NAMES,
        &["order_value"]
    );
    assert_eq!(
        <OrdersQueryModel as PersistedFields>::FIELD_NAMES,
        &["order_value", "order_number"]
    );
}

#[test]
fn test_conditions_are_empty_without_options() {
    assert!(TicketsFilterOptionsModel::default().conditions().is_empty());
}

#[test]
fn test_zero_sized_enums_are_generated_when_no_sortable_provided() {
    assert_eq!(std::mem::size_of::<UserSortFieldModel>(), 0);
}

#[test]
fn test_sized_enums_are_generated_when_sortable_provided() {
    assert_eq!(std::mem::size_of::<OrderSortFieldModel>(), 1);
}
