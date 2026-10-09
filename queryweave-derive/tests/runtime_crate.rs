//! Every derive's generated code compiles and works when its runtime crate is
//! reached through another path, given by `#[queryweave(crate = path)]`: here,
//! modules re-exporting the runtime crates, as a crate wrapping `queryweave`
//! would.

#![allow(dead_code, clippy::option_option)]

use pretty_assertions::assert_eq;
use queryweave_derive::{
    AppendToQuery, AppendToQueryWithSeparator, Columns, Filters, MergePatch,
    MergePatchNullableFields, RawValue, ValidateNonNullable,
};
use sqlx::{Postgres, QueryBuilder};
use validator::Validate;

/// The core runtime, re-exported.
mod core_runtime {
    pub use queryweave::*;
}

/// The API runtime (with the core), re-exported.
mod api_runtime {
    pub use queryweave_api::*;
}

/// The SQL runtime (with the core), re-exported.
mod sql_runtime {
    pub use queryweave_sql::*;
}

#[derive(Debug, Clone, PartialEq, RawValue)]
#[queryweave(crate = crate::core_runtime)]
struct Label<T = String>(T);

#[derive(Debug, Clone, PartialEq, Filters, MergePatch)]
#[queryweave(crate = crate::core_runtime)]
#[merge_patch(patch = MergePatchWidgetModel)]
struct WidgetModel {
    #[filter(sw)]
    #[sortable]
    label: Label<String>,

    note: Option<String>,
}

#[derive(Default, MergePatchNullableFields)]
#[queryweave(crate = crate::core_runtime)]
struct MergePatchWidgetModel {
    label: Option<Label<String>>,
    note: queryweave::Patch<String>,
}

#[derive(Debug, serde::Serialize, Filters)]
#[queryweave(crate = crate::api_runtime)]
#[filters(mirrors = WidgetModel, serde, utoipa, validator)]
struct WidgetDto {
    #[filter(sw)]
    #[sortable]
    label: String,
}

#[derive(Validate, ValidateNonNullable)]
#[queryweave(crate = crate::api_runtime)]
#[validate_non_nullable(mirrors = MergePatchWidgetModel, validator)]
struct MergePatchWidgetDto {
    label: queryweave::Patch<String>,
    note: queryweave::Patch<String>,
}

#[derive(Columns)]
#[queryweave(crate = crate::sql_runtime)]
#[columns(model = WidgetModel)]
struct Widget {
    label: String,
    note: Option<String>,
}

#[derive(Debug, AppendToQueryWithSeparator)]
#[queryweave(crate = crate::sql_runtime)]
#[append_to_query(columns = Widget)]
struct WidgetsFilterOptionsModelWrapper(WidgetsFilterOptionsModel);

#[derive(Debug, AppendToQuery)]
#[queryweave(crate = crate::sql_runtime)]
#[append_to_query(columns = Widget, filters = WidgetsFilterOptionsModelWrapper)]
struct WidgetsQueryModelWrapper(WidgetsQueryModel);

type NoGuard = Option<fn(&mut QueryBuilder<'_, Postgres>, &WidgetsQueryModelWrapper)>;

// The mirror converts into the root model's query, which the wrappers write
// as SQL: every derive works through the re-exported runtimes.
#[test]
fn test_the_derives_work_through_re_exported_runtime_crates() {
    let query_params: WidgetsQueryParams = serde_json::from_value(serde_json::json!({
        "label_sw": "ab",
        "sort": [{ "field": "label", "order": "descending" }],
    }))
    .unwrap();
    let query: WidgetsQueryModel = query_params.into();

    let mut query_builder = QueryBuilder::<Postgres>::new("SELECT * FROM widgets ");
    WidgetsQueryModelWrapper(query).append_to_query(&mut query_builder, NoGuard::None);

    assert_eq!(
        query_builder.sql(),
        "SELECT * FROM widgets WHERE label LIKE $1 ESCAPE '!' ORDER BY label DESC "
    );
}

#[test]
fn test_merge_patching_works_through_re_exported_runtime_crates() {
    let mut widget = WidgetModel {
        label: Label(String::from("old")),
        note: Some(String::from("note")),
    };

    widget.merge(MergePatchWidgetModel {
        label: Some(Label(String::from("new"))),
        note: queryweave::Patch::Null,
    });

    assert_eq!(
        widget,
        WidgetModel {
            label: Label(String::from("new")),
            note: None,
        }
    );
    assert!(
        MergePatchWidgetDto {
            label: queryweave::Patch::Null,
            note: queryweave::Patch::Missing,
        }
        .validate_non_nullable()
        .is_err()
    );
}
