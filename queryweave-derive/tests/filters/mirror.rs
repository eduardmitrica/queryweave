//! `Filters` on structs mirroring a root model.

use queryweave_derive::Filters;

#[allow(dead_code)]
#[derive(Filters)]
struct UserModel {
    #[filter(sw)]
    full_name: String,
}

#[allow(dead_code)]
#[derive(Filters)]
#[filters(mirrors = UserModel, serde, utoipa, validator)]
struct UserDto {
    #[filter(sw)]
    full_name: String,
}

#[allow(dead_code)]
#[derive(Filters)]
struct OrderModel {
    #[filter(lt)]
    #[sortable]
    value: i32,
}

#[allow(dead_code)]
#[derive(Filters)]
#[filters(mirrors = OrderModel, serde, utoipa, validator)]
struct OrderDto {
    #[filter(lt)]
    #[sortable]
    value: i32,
}

// An unsortable DTO filtering on a float. Its query params used to derive
// `Eq`, which `Option<f64>` does not implement, so this failed to compile.
#[allow(dead_code)]
#[derive(Filters)]
struct ProductModel {
    #[filter(lt)]
    price: f64,
}

#[allow(dead_code)]
#[derive(Filters)]
#[filters(mirrors = ProductModel, serde, utoipa, validator)]
struct ProductDto {
    #[filter(lt)]
    price: f64,
}

#[test]
fn test_unsortable_query_params_allow_float_filters() {
    let query_params: ProductsQueryParams =
        serde_json::from_value(serde_json::json!({ "price_lt": 9.5 })).unwrap();

    assert_eq!(query_params.price_lt, Some(9.5));
}

// A DTO whose field is named differently on the domain model maps onto it with
// `#[equivalent_field(name)]`; its query parameters keep the DTO's name.
#[allow(dead_code)]
#[derive(Filters)]
struct InvoiceModel {
    #[filter(eq)]
    #[sortable]
    number: String,
}

#[allow(dead_code)]
#[derive(Filters)]
#[filters(mirrors = InvoiceModel, serde, utoipa, validator)]
struct InvoiceDto {
    #[filter(eq)]
    #[sortable]
    #[equivalent_field(number)]
    invoice_number: String,
}

#[test]
fn test_a_renamed_filter_fills_the_equivalent_domain_filter() {
    let query_params: InvoicesQueryParams =
        serde_json::from_value(serde_json::json!({ "invoice_number_eq": "INV-1" })).unwrap();

    let filters = InvoicesFilterOptionsModel::from(query_params);

    assert_eq!(filters.number_eq, Some(String::from("INV-1")));
}

#[test]
fn test_a_renamed_sort_field_maps_onto_the_equivalent_domain_variant() {
    assert_eq!(
        InvoiceSortFieldModel::from(InvoiceSortFieldDto::InvoiceNumber),
        InvoiceSortFieldModel::Number
    );
    assert_eq!(
        <InvoiceSortFieldDto as queryweave::SortFieldOf<InvoiceModel>>::from_sort_field(
            InvoiceSortFieldModel::Number
        ),
        InvoiceSortFieldDto::InvoiceNumber
    );
}

// The DTO links to its model through `queryweave::Filterable`, not by
// rebuilding the model's generated names from its own, so the two names need
// not share a stem (`CustomersQueryRecord` vs `AccountsQueryParams`).
#[allow(dead_code)]
#[derive(Filters)]
struct CustomerRecord {
    #[filter(sw)]
    #[sortable]
    name: String,
}

#[allow(dead_code)]
#[derive(Filters)]
#[filters(mirrors = CustomerRecord, serde, utoipa, validator)]
struct AccountDto {
    #[filter(sw)]
    #[sortable]
    name: String,
}

#[test]
fn test_a_dto_converts_into_a_differently_named_model() {
    let query_params: AccountsQueryParams = serde_json::from_value(serde_json::json!({
        "name_sw": "Ad",
        "sort": [{ "field": "name", "order": "descending" }],
    }))
    .unwrap();

    let query = CustomersQueryRecord::from(query_params);

    assert_eq!(query.and.name_sw, Some(String::from("Ad")));
    let sort = query.sort.unwrap();
    assert_eq!(sort.len(), 1);
    assert_eq!(sort[0].field, CustomerSortFieldRecord::Name);
    assert_eq!(sort[0].order, queryweave::SortDirectionModel::Descending);
}

// Both the unsortable and the sortable query params reject a zero page size
// (it used to reach a division by zero) and accept any positive one or none.
#[test]
fn test_unsortable_query_params_validate_the_page_size() {
    use validator::Validate;

    let with_page_size = |page_size| UsersQueryParams {
        page_size,
        ..Default::default()
    };

    assert!(with_page_size(Some(0)).validate().is_err());
    assert!(with_page_size(Some(1)).validate().is_ok());
    assert!(with_page_size(None).validate().is_ok());
}

#[test]
fn test_sortable_query_params_validate_the_page_size() {
    use validator::Validate;

    let with_page_size = |page_size| OrdersQueryParams {
        page_size,
        ..Default::default()
    };

    assert!(with_page_size(Some(0)).validate().is_err());
    assert!(with_page_size(Some(1)).validate().is_ok());
    assert!(with_page_size(None).validate().is_ok());
}

#[test]
fn test_zero_sized_enums_are_generated_when_no_sortable_provided() {
    assert_eq!(std::mem::size_of::<UserSortFieldDto>(), 0);
    assert_eq!(std::mem::size_of::<UserSortFieldModel>(), 0);
}

#[test]
fn test_query_params() {
    let value = serde_json::json!({
        "page": 1,
        "page_size": 3,
        "full_name_sw": "A",
    });
    let query_params: UsersQueryParams = serde_json::from_value(value).unwrap();

    insta::assert_debug_snapshot!(query_params);
}
