//! `Filters` with some generated types renamed: the renamed types link like
//! any other, since layers name each other's types through `Filterable`.

use queryweave_derive::Filters;

#[allow(dead_code)]
#[derive(Filters)]
#[filters(query = UserSearch, sort_field = UserSortKey)]
pub struct UserModel {
    #[filter(sw)]
    #[sortable]
    pub username: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Filters)]
#[filters(mirrors = UserModel, serde, query_params = UserSearchParams, page = UserPage)]
pub struct UserDto {
    #[filter(sw)]
    #[sortable]
    pub username: String,
}

// The renamed types replace the conventions' names; the others keep theirs.
#[test]
fn test_renamed_types_link_like_any_other() {
    let params = UserSearchParams {
        username_sw: Some(String::from("ad")),
        sort: Some(vec![UserSortDetailsDto {
            field: UserSortFieldDto::Username,
            order: queryweave_api::SortDirectionDto::Ascending,
        }]),
        ..Default::default()
    };

    let query: UserSearch = params.into();

    assert_eq!(query.and.username_sw.as_deref(), Some("ad"));
    assert_eq!(query.sort.unwrap()[0].field, UserSortKey::Username);
    let _: Option<UserPage> = None;
}
