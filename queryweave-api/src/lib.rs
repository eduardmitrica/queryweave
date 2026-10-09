//! The `queryweave` runtime for the API layer: the types the code generated
//! for API types (a `Filters` mirror with `serde`, `utoipa` or `schemars`,
//! `ValidateNonNullable`) names, on top of the core runtime (`queryweave`),
//! which it re-exports so that code names only this crate.
//!
//! Every integration is a feature, none enabled by default: `serde`
//! (`Deserialize` / `Serialize`), `utoipa` (`ToSchema`) and `schemars`
//! (`JsonSchema`) add their derives to [`SortDirectionDto`] and [`PageDto`].
//! A mirror's flag needs the feature of the same name, and reports it
//! disabled at the flag (see [`features`]). Each integration states the wire
//! format itself (snake case directions), so they agree whichever are
//! enabled.

pub use queryweave::*;

pub mod features;
mod page;

pub use page::PageDto;

/// The sort direction applied to the result retrieved by filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[cfg_attr(
    feature = "utoipa",
    derive(utoipa::ToSchema),
    schema(rename_all = "snake_case")
)]
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(rename_all = "snake_case")
)]
pub enum SortDirectionDto {
    /// Sort in ascending order.
    #[default]
    Ascending,
    /// Sort in descending order.
    Descending,
}

impl From<SortDirectionDto> for queryweave::SortDirectionModel {
    fn from(value: SortDirectionDto) -> Self {
        match value {
            SortDirectionDto::Ascending => Self::Ascending,
            SortDirectionDto::Descending => Self::Descending,
        }
    }
}

impl From<queryweave::SortDirectionModel> for SortDirectionDto {
    fn from(value: queryweave::SortDirectionModel) -> Self {
        match value {
            queryweave::SortDirectionModel::Ascending => Self::Ascending,
            queryweave::SortDirectionModel::Descending => Self::Descending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PageDto;

    fn total_pages(page_size: Option<u32>, total_items: u32) -> Option<u32> {
        PageDto::<u32>::new(Some(0), page_size, total_items, Vec::new()).total_pages
    }

    // A partial last page still counts as a page.
    #[test]
    fn total_pages_rounds_up() {
        assert_eq!(total_pages(Some(10), 0), Some(0));
        assert_eq!(total_pages(Some(10), 10), Some(1));
        assert_eq!(total_pages(Some(10), 11), Some(2));
    }

    // Without a page size there is nothing to count pages by.
    #[test]
    fn total_pages_is_none_without_a_page_size() {
        assert_eq!(total_pages(None, 11), None);
    }

    // A zero page size used to divide by zero and panic.
    #[test]
    fn total_pages_is_none_for_a_zero_page_size() {
        assert_eq!(total_pages(Some(0), 11), None);
    }

    // serde (de)serializes a direction in snake case.
    #[cfg(feature = "serde")]
    #[test]
    fn serde_writes_directions_in_snake_case() {
        use super::SortDirectionDto;

        assert_eq!(
            serde_json::to_value(SortDirectionDto::Descending).unwrap(),
            serde_json::json!("descending")
        );
        assert_eq!(
            serde_json::from_value::<SortDirectionDto>(serde_json::json!("ascending")).unwrap(),
            SortDirectionDto::Ascending
        );
    }

    // schemars describes the same snake case values, whether or not serde is
    // enabled.
    #[cfg(feature = "schemars")]
    #[test]
    fn schemars_describes_directions_in_snake_case() {
        let schema = serde_json::to_value(schemars::schema_for!(super::SortDirectionDto)).unwrap();
        // A documented variant is a `oneOf` constant.
        let values = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|variant| variant["const"].clone())
            .collect::<Vec<_>>();

        assert_eq!(values, ["ascending", "descending"]);
    }

    // utoipa documents the same snake case values, whether or not serde is
    // enabled (it states them itself rather than reading serde's attributes).
    #[cfg(feature = "utoipa")]
    #[test]
    fn utoipa_documents_directions_in_snake_case() {
        let schema =
            serde_json::to_value(<super::SortDirectionDto as utoipa::PartialSchema>::schema())
                .unwrap();

        assert_eq!(
            schema["enum"],
            serde_json::json!(["ascending", "descending"])
        );
    }
}
