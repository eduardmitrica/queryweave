//! Each integration of a mirror on its own: none relies on another's
//! attributes, so each states the wire format itself.

use queryweave_derive::Filters;

#[allow(dead_code)]
#[derive(Filters)]
pub struct ArticleModel {
    #[filter(sw)]
    #[sortable]
    pub full_title: String,
}

#[allow(dead_code)]
#[derive(Filters)]
pub struct TagModel {
    #[filter(eq)]
    pub label: String,
}

/// Two sortable fields: a repeated sort fits the length rule, so only the
/// uniqueness rule rejects it.
#[allow(dead_code)]
#[derive(Filters)]
pub struct BookModel {
    #[filter(sw)]
    #[sortable]
    pub title: String,

    #[filter(gt)]
    #[sortable]
    pub pages: i32,
}

/// Documented without serde.
mod utoipa_only {
    use queryweave_derive::Filters;

    #[allow(dead_code)]
    #[derive(Filters)]
    #[filters(mirrors = super::ArticleModel, utoipa)]
    pub struct ArticleDto {
        #[filter(sw)]
        #[sortable]
        pub full_title: String,
    }

    #[allow(dead_code)]
    #[derive(Filters)]
    #[filters(mirrors = super::TagModel, utoipa)]
    pub struct TagDto {
        #[filter(eq)]
        pub label: String,
    }
}

/// Deserialized without utoipa.
mod serde_only {
    use queryweave_derive::Filters;

    #[allow(dead_code)]
    #[derive(Debug, serde::Serialize, Filters)]
    #[filters(mirrors = super::ArticleModel, serde)]
    pub struct ArticleDto {
        #[filter(sw)]
        #[sortable]
        pub full_title: String,
    }
}

/// Validated by garde, with neither serde nor validator.
mod garde_only {
    use queryweave_derive::Filters;

    #[allow(dead_code)]
    #[derive(Filters)]
    #[filters(mirrors = super::ArticleModel, garde)]
    pub struct ArticleDto {
        #[filter(sw)]
        #[sortable]
        pub full_title: String,
    }

    #[allow(dead_code)]
    #[derive(Filters)]
    #[filters(mirrors = super::TagModel, garde)]
    pub struct TagDto {
        #[filter(eq)]
        pub label: String,
    }

    #[allow(dead_code)]
    #[derive(Filters)]
    #[filters(mirrors = super::BookModel, garde)]
    pub struct BookDto {
        #[filter(sw)]
        #[sortable]
        pub title: String,

        #[filter(gt)]
        #[sortable]
        pub pages: i32,
    }
}

/// Described by schemars, without serde.
mod schemars_only {
    use queryweave_derive::Filters;

    #[allow(dead_code)]
    #[derive(Filters)]
    #[filters(mirrors = super::ArticleModel, schemars)]
    pub struct ArticleDto {
        #[filter(sw)]
        #[sortable]
        pub full_title: String,
    }

    #[allow(dead_code)]
    #[derive(Filters)]
    #[filters(mirrors = super::TagModel, schemars)]
    pub struct TagDto {
        #[filter(eq)]
        pub label: String,
    }
}

fn schema<T: utoipa::PartialSchema>() -> serde_json::Result<serde_json::Value> {
    serde_json::to_value(T::schema())
}

fn parameter_names<T: utoipa::IntoParams>() -> Vec<String> {
    T::into_params(|| None)
        .into_iter()
        .map(|parameter| parameter.name)
        .collect()
}

// utoipa documents the sort variants in snake case without serde's
// `rename_all`, as serde (de)serializes them.
#[test]
fn test_utoipa_alone_documents_snake_case_sort_variants() {
    assert_eq!(
        schema::<utoipa_only::ArticleSortFieldDto>().unwrap()["enum"],
        serde_json::json!(["full_title"])
    );
}

#[test]
fn test_serde_alone_reads_snake_case_sort_variants() {
    let query_params: serde_only::ArticlesQueryParams = serde_json::from_value(serde_json::json!({
        "sort": [{ "field": "full_title", "order": "descending" }],
    }))
    .unwrap();

    assert_eq!(
        query_params.sort.unwrap()[0].field,
        serde_only::ArticleSortFieldDto::FullTitle
    );
}

// An unsortable mirror has no `sort` parameter to document (nor to skip with
// serde's attributes), while a sortable one documents it.
#[test]
fn test_utoipa_documents_sort_only_on_a_sortable_mirror() {
    assert_eq!(
        parameter_names::<utoipa_only::TagsQueryParams>(),
        ["page", "page_size", "label_eq"]
    );
    assert!(parameter_names::<utoipa_only::ArticlesQueryParams>().contains(&String::from("sort")));
}

/// A garde-validated article query sorting by `fields`, of `page_size`.
fn garde_query(
    page_size: Option<u32>,
    fields: &[garde_only::ArticleSortFieldDto],
) -> garde_only::ArticlesQueryParams {
    garde_only::ArticlesQueryParams {
        page_size,
        sort: Some(
            fields
                .iter()
                .map(|&field| garde_only::ArticleSortDetailsDto {
                    field,
                    order: queryweave::SortDirectionModel::Ascending,
                })
                .collect(),
        ),
        ..Default::default()
    }
}

// garde validates the page size and the sort like validator does, without a
// serializer (its sort direction is the core one).
#[test]
fn test_garde_validates_the_page_size_and_the_sort() {
    use garde::Validate;
    use garde_only::ArticleSortFieldDto::FullTitle;

    assert!(garde_query(Some(1), &[FullTitle]).validate().is_ok());
    assert!(garde_query(Some(0), &[FullTitle]).validate().is_err());
    // Empty, or repeated (here also longer than the one sortable field).
    assert!(garde_query(None, &[]).validate().is_err());
    assert!(
        garde_query(None, &[FullTitle, FullTitle])
            .validate()
            .is_err()
    );
    // Without a sort, and on an unsortable mirror, only the page size counts.
    assert!(
        garde_only::ArticlesQueryParams::default()
            .validate()
            .is_ok()
    );
    assert!(
        garde_only::TagsQueryParams {
            page_size: Some(0),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}

// garde rejects a sort repeating a field, even within the length limit.
#[test]
fn test_garde_rejects_a_repeated_sort_field() {
    use garde::Validate;
    use garde_only::BookSortFieldDto::{Pages, Title};

    let query = |fields: &[garde_only::BookSortFieldDto]| garde_only::BooksQueryParams {
        sort: Some(
            fields
                .iter()
                .map(|&field| garde_only::BookSortDetailsDto {
                    field,
                    order: queryweave::SortDirectionModel::Descending,
                })
                .collect(),
        ),
        ..Default::default()
    };

    assert!(query(&[Title, Pages]).validate().is_ok());
    assert!(query(&[Title, Title]).validate().is_err());
    assert!(query(&[Title, Pages, Title]).validate().is_err());
}

/// The JSON schema of `T`.
fn json_schema<T: schemars::JsonSchema>() -> serde_json::Result<serde_json::Value> {
    serde_json::to_value(schemars::schema_for!(T))
}

// schemars describes the sort variants in snake case without serde, and the
// `sort` parameter only on a sortable mirror.
#[test]
fn test_schemars_alone_describes_snake_case_sort_variants() {
    let sort_field = json_schema::<schemars_only::ArticleSortFieldDto>().unwrap();
    // A documented variant is a `oneOf` constant.
    let variants = sort_field["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| variant["const"].clone())
        .collect::<Vec<_>>();
    assert_eq!(variants, ["full_title"]);

    let sortable = json_schema::<schemars_only::ArticlesQueryParams>().unwrap();
    let unsortable = json_schema::<schemars_only::TagsQueryParams>().unwrap();
    assert!(sortable["properties"].get("sort").is_some());
    assert!(unsortable["properties"].get("sort").is_none());
    assert!(unsortable["properties"].get("label_eq").is_some());
}
