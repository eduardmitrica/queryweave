//! `Filters` on a chain of mirrors: a gRPC message mirroring an API DTO
//! mirroring a domain model, each layer renaming the field it filters and
//! sorts by (`label` → `title` → `name`). The message is a plain mirror (no
//! serde, utoipa or validator), the DTO opts into all three.

mod domain {
    use queryweave_derive::Filters;

    #[allow(dead_code)]
    #[derive(Filters)]
    pub struct ProductModel {
        #[filter(sw)]
        #[sortable]
        pub name: String,

        #[filter(lt)]
        pub price: f64,

        #[filter(bool)]
        pub active: bool,
    }
}

mod api {
    use queryweave_derive::Filters;

    #[allow(dead_code)]
    #[derive(Debug, serde::Serialize, Filters)]
    #[filters(mirrors = super::domain::ProductModel, serde, utoipa, validator)]
    pub struct ProductDto {
        #[filter(sw)]
        #[sortable]
        #[equivalent_field(name)]
        pub title: String,

        #[filter(lt)]
        pub price: f64,

        #[filter(bool)]
        pub active: bool,
    }
}

mod grpc {
    use queryweave_derive::Filters;

    // Mirrors the API DTO, not the domain model: its names map onto the DTO's.
    #[allow(dead_code)]
    #[derive(Debug, Filters)]
    #[filters(mirrors = super::api::ProductDto)]
    pub struct ProductMessage {
        #[filter(sw)]
        #[sortable]
        #[equivalent_field(title)]
        pub label: String,

        #[filter(lt)]
        pub price: f64,

        #[filter(bool)]
        pub active: bool,
    }
}

// Built by hand: a plain mirror is not deserializable. Its direction is the
// core `SortDirectionModel`, converted into the DTO's `SortDirectionDto`.
fn message_query() -> grpc::ProductsQueryParams {
    grpc::ProductsQueryParams {
        page: Some(1),
        label_sw: Some(String::from("Ab")),
        price_lt: Some(10.5),
        active: Some(true),
        sort: Some(vec![grpc::ProductSortDetailsMessage {
            field: grpc::ProductSortFieldMessage::Label,
            order: queryweave::SortDirectionModel::Descending,
        }]),
        ..Default::default()
    }
}

// One hop at a time, each with `.into()`: message → DTO → domain query.
#[test]
fn test_each_hop_converts_into_the_mirrored_struct() {
    let dto: api::ProductsQueryParams = message_query().into();

    assert_eq!(dto.page, Some(1));
    assert_eq!(dto.title_sw.as_deref(), Some("Ab"));
    assert_eq!(dto.price_lt, Some(10.5));
    assert_eq!(dto.active, Some(true));
    let sort = dto.sort.clone().unwrap();
    assert_eq!(sort[0].field, api::ProductSortFieldDto::Title);
    assert_eq!(sort[0].order, queryweave_api::SortDirectionDto::Descending);

    let query: domain::ProductsQueryModel = dto.into();

    assert_eq!(query.and.name_sw.as_deref(), Some("Ab"));
    assert_eq!(
        query.sort.unwrap()[0].field,
        domain::ProductSortFieldModel::Name
    );
}

// Straight to the root model's query, whatever the chain's length.
#[test]
fn test_into_root_query_follows_the_whole_chain() {
    use queryweave::IntoRootQuery;

    let query: domain::ProductsQueryModel = message_query().into_root_query();

    assert_eq!(query.page, Some(1));
    assert_eq!(query.and.name_sw.as_deref(), Some("Ab"));
    assert_eq!(query.and.price_lt, Some(10.5));
    assert_eq!(query.and.active, Some(true));
    let sort = query.sort.unwrap();
    assert_eq!(sort[0].field, domain::ProductSortFieldModel::Name);
    assert_eq!(sort[0].order, queryweave::SortDirectionModel::Descending);
}

// The root model's query converts into itself.
#[test]
fn test_a_root_query_is_its_own_root_query() {
    use queryweave::IntoRootQuery;

    let query = domain::ProductsQueryModel {
        page: Some(3),
        ..Default::default()
    };

    assert_eq!(query.into_root_query().page, Some(3));
}

// The root of every link is the domain model, by type.
#[test]
fn test_every_link_has_the_domain_model_as_root() {
    fn root_of<M: queryweave::Filterable<Root = domain::ProductModel>>() {}

    root_of::<domain::ProductModel>();
    root_of::<api::ProductDto>();
    root_of::<grpc::ProductMessage>();
}

// A sort field maps both ways along a link.
#[test]
fn test_a_sort_field_maps_both_ways_along_a_link() {
    use queryweave::SortFieldOf;

    let dto: api::ProductSortFieldDto = grpc::ProductSortFieldMessage::Label.into();
    assert_eq!(dto, api::ProductSortFieldDto::Title);
    assert_eq!(
        <grpc::ProductSortFieldMessage as SortFieldOf<api::ProductDto>>::from_sort_field(dto),
        grpc::ProductSortFieldMessage::Label
    );
}
