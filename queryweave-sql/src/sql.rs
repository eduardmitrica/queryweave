//! The SQL written for the domain's database-agnostic data, portable across
//! the databases sqlx supports (Postgres, MySQL/MariaDB, SQLite).
//!
//! Only standard SQL is written: comparisons, `LIKE … ESCAPE`, `ORDER BY`,
//! `col = value` assignments and `LIMIT … OFFSET …`. Placeholders come from
//! the driver's [`QueryBuilder`] (`$1` on Postgres, `?` elsewhere), and every
//! value is bound, never interpolated.

use crate::{BindValues, Columns};
use queryweave::{
    Conditions, FilterCondition, FilterOperator, SortDirectionModel, SortKey, SortKeys, Value,
};
#[cfg(feature = "merge-patch")]
use queryweave::{FieldValue, FieldValues};
use sqlx::QueryBuilder;
#[cfg(feature = "merge-patch")]
use sqlx::query_builder::Separated;

/// The character escaping the `LIKE` metacharacters in a bound pattern.
///
/// Not the backslash: MySQL reads `'\'` as an escape inside string literals
/// (so `ESCAPE '\'` does not even parse), and SQLite has no default escape
/// character at all. `!` means the same everywhere.
const LIKE_ESCAPE: char = '!';

/// The SQL comparison operator for a filter operator.
#[must_use]
pub const fn sql_operator(operator: FilterOperator) -> &'static str {
    match operator {
        FilterOperator::LessThan => "<",
        FilterOperator::Equal => "=",
        FilterOperator::GreaterThan => ">",
        FilterOperator::StartsWith | FilterOperator::Contains | FilterOperator::EndsWith => "LIKE",
    }
}

/// The SQL keyword for a sort direction.
#[must_use]
pub const fn sql_direction(direction: SortDirectionModel) -> &'static str {
    match direction {
        SortDirectionModel::Ascending => "ASC",
        SortDirectionModel::Descending => "DESC",
    }
}

/// The `LIKE` pattern to bind for a `LIKE` operator: the escaped text wrapped
/// in `%` wildcards. `None` for the comparison operators, which bind the value
/// itself.
///
/// Kept separate so the wildcard placement can be unit-tested directly: the
/// value is pushed as a bound parameter and therefore never appears in the
/// rendered SQL.
pub fn like_pattern(operator: FilterOperator, text: &str) -> Option<String> {
    let text = escape_like(text);

    match operator {
        FilterOperator::StartsWith => Some(format!("{text}%")),
        FilterOperator::Contains => Some(format!("%{text}%")),
        FilterOperator::EndsWith => Some(format!("%{text}")),
        FilterOperator::LessThan | FilterOperator::Equal | FilterOperator::GreaterThan => None,
    }
}

/// Escape the `LIKE` metacharacters (`%`, `_` and [`LIKE_ESCAPE`] itself) so
/// user input matches literally instead of acting as wildcards.
pub fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        if matches!(character, '%' | '_') || character == LIKE_ESCAPE {
            escaped.push(LIKE_ESCAPE);
        }
        escaped.push(character);
    }

    escaped
}

/// The column `C` stores a domain field in.
///
/// The writers below take the data of a model `M` (a
/// `queryweave::ModelData<M, _>`, whose items only name fields of `M`), and
/// `Columns`' methods check at compile time that every field of `M` has a
/// column on `C`, so the lookup cannot miss. The fallback to the field's own
/// name only satisfies the types (a runtime panic is not allowed here).
fn column<C: Columns + ?Sized>(field: &'static str) -> &'static str {
    C::column(field).unwrap_or(field)
}

/// Push a `column operator value` comparison and bind the value, the column
/// being the one `C` stores the condition's domain field in.
///
/// Comparisons bind the value with its own SQL type (binding its text form
/// would compare e.g. an integer column against text, which Postgres
/// rejects); the `LIKE` operators bind an escaped `%` pattern of the text and
/// name the escape character.
fn push_condition<C: Columns + ?Sized, DB: BindValues>(
    query_builder: &mut QueryBuilder<'_, DB>,
    condition: FilterCondition,
) {
    let FilterCondition {
        field,
        operator,
        value,
    } = condition;

    query_builder.push(format!(
        "{} {} ",
        column::<C>(field),
        sql_operator(operator)
    ));

    // The `Text` filter kind guarantees the `LIKE` operators only ever carry
    // text; anything else is bound as it is.
    match value {
        Value::Text(Some(text)) => match like_pattern(operator, &text) {
            Some(pattern) => {
                DB::push_bind_value(query_builder, Value::Text(Some(pattern)));
                query_builder.push(format!(" ESCAPE '{LIKE_ESCAPE}'"));
            },
            None => DB::push_bind_value(query_builder, Value::Text(Some(text))),
        },
        value => DB::push_bind_value(query_builder, value),
    }
    query_builder.push(" ");
}

/// See [`Columns::push_conditions`].
pub fn push_conditions<C: Columns + ?Sized, DB: BindValues, M>(
    query_builder: &mut QueryBuilder<'_, DB>,
    conditions: Conditions<M>,
    separator: &str,
) {
    for (index, condition) in conditions.into_iter().enumerate() {
        if index > 0 {
            query_builder.push(format!("{separator} "));
        }
        push_condition::<C, DB>(query_builder, condition);
    }
}

/// See [`Columns::push_order_by`].
pub fn push_order_by<C: Columns + ?Sized, DB: BindValues, M>(
    query_builder: &mut QueryBuilder<'_, DB>,
    sort_keys: SortKeys<M>,
) {
    if sort_keys.is_empty() {
        return;
    }

    let order_by = sort_keys
        .into_iter()
        .map(|SortKey { field, direction }| {
            format!("{} {}", column::<C>(field), sql_direction(direction))
        })
        .collect::<Vec<_>>()
        .join(", ");
    query_builder.push(format!("ORDER BY {order_by} "));
}

/// See [`Columns::push_assignments`].
#[cfg(feature = "merge-patch")]
pub fn push_assignments<'qb, 'args, C: Columns + ?Sized, DB: BindValues, M>(
    query_builder: &'qb mut QueryBuilder<'args, DB>,
    assignments: FieldValues<M>,
) -> Separated<'qb, 'args, DB, &'static str> {
    let mut separated = query_builder.separated(", ");

    for FieldValue { field, value } in assignments {
        separated.push(format!("{} = ", column::<C>(field)));
        DB::push_bind_value_unseparated(&mut separated, value);
    }

    separated
}

/// The row offset of page `page` (0 based) of `page_size` rows, saturated at
/// `i64::MAX`: `u32 * u32` always fits a `u64`, and an offset past every
/// database's signed 64-bit row count just yields an empty page.
fn offset(page: u32, page_size: u32) -> i64 {
    i64::try_from(u64::from(page) * u64::from(page_size)).unwrap_or(i64::MAX)
}

/// Push a `LIMIT page_size OFFSET page * page_size` clause, binding both, when
/// both a page and a page size are requested; nothing otherwise.
pub fn push_page<DB: BindValues>(
    query_builder: &mut QueryBuilder<'_, DB>,
    page: Option<u32>,
    page_size: Option<u32>,
) {
    let (Some(page), Some(page_size)) = (page, page_size) else {
        return;
    };

    query_builder.push("LIMIT ");
    DB::push_bind_value(query_builder, Value::I64(Some(i64::from(page_size))));
    query_builder.push(" OFFSET ");
    DB::push_bind_value(query_builder, Value::I64(Some(offset(page, page_size))));
}

#[cfg(test)]
mod tests {
    use super::{escape_like, like_pattern, offset};
    use crate::{BindValues, Columns, push_page};
    use queryweave::{
        FieldValue, FilterCondition, FilterOperator, PersistedFields, SortDirectionModel, SortKey,
        Value,
    };
    use rstest::rstest;
    use sqlx::{MySql, Postgres, QueryBuilder, Sqlite};

    /// Each field stored in the column of its name, except `delivery_date`,
    /// stored in `delivered_at`.
    struct Table;

    impl Columns for Table {
        const NAME: &'static str = "Table";
        const COLUMNS: &'static [(&'static str, &'static str)] = &[
            ("number", "number"),
            ("client", "client"),
            ("capacity", "capacity"),
            ("value", "value"),
            ("is_closed", "is_closed"),
            ("user_count", "user_count"),
            ("is_sweet", "is_sweet"),
            ("is_equal_to", "is_equal_to"),
            ("name_c", "name_c"),
            ("delivery_date", "delivered_at"),
        ];
    }

    /// A model whose every field has a column on [`Table`].
    struct Model;

    impl PersistedFields for Model {
        const FIELD_NAMES: &'static [&'static str] = &[
            "number",
            "client",
            "capacity",
            "value",
            "is_closed",
            "user_count",
            "is_sweet",
            "is_equal_to",
            "name_c",
            "delivery_date",
        ];
    }

    fn condition(field: &'static str, operator: FilterOperator) -> FilterCondition {
        FilterCondition {
            field,
            operator,
            value: Value::Text(Some(String::from("needle"))),
        }
    }

    fn conditions_sql<DB: BindValues>(conditions: Vec<FilterCondition>) -> String {
        let mut query_builder = QueryBuilder::<DB>::default();
        Table::push_conditions(
            &mut query_builder,
            queryweave::__private::model_data::<Model, _>(conditions),
            "AND",
        );
        query_builder.sql().to_string()
    }

    fn order_by_sql<DB: BindValues>(sort_keys: Vec<SortKey>) -> String {
        let mut query_builder = QueryBuilder::<DB>::default();
        Table::push_order_by(
            &mut query_builder,
            queryweave::__private::model_data::<Model, _>(sort_keys),
        );
        query_builder.sql().to_string()
    }

    fn page_sql<DB: BindValues>(page: Option<u32>, page_size: Option<u32>) -> String {
        let mut query_builder = QueryBuilder::<DB>::default();
        push_page(&mut query_builder, page, page_size);
        query_builder.sql().to_string()
    }

    /// Conditions, sorting and a page, as one `WHERE … ORDER BY … LIMIT …`
    /// tail.
    fn query_tail_sql<DB: BindValues>() -> String {
        let mut query_builder = QueryBuilder::<DB>::default();
        Table::push_conditions(
            &mut query_builder,
            queryweave::__private::model_data::<Model, _>(vec![
                condition("number", FilterOperator::Equal),
                condition("client", FilterOperator::StartsWith),
            ]),
            "AND",
        );
        Table::push_order_by(
            &mut query_builder,
            queryweave::__private::model_data::<Model, _>(vec![SortKey {
                field: "delivery_date",
                direction: SortDirectionModel::Descending,
            }]),
        );
        push_page(&mut query_builder, Some(2), Some(10));
        query_builder.sql().to_string()
    }

    // The same data renders the same SQL on every database, with its own
    // placeholders.
    #[test]
    fn every_database_gets_the_same_sql_with_its_own_placeholders() {
        assert_eq!(
            query_tail_sql::<Postgres>(),
            "number = $1 AND client LIKE $2 ESCAPE '!' ORDER BY delivered_at DESC LIMIT $3 OFFSET $4"
        );
        let with_question_marks =
            "number = ? AND client LIKE ? ESCAPE '!' ORDER BY delivered_at DESC LIMIT ? OFFSET ?";
        assert_eq!(query_tail_sql::<MySql>(), with_question_marks);
        assert_eq!(query_tail_sql::<Sqlite>(), with_question_marks);
    }

    #[test]
    fn assignments_render_on_every_database() {
        fn assignments_sql<DB: BindValues>() -> String {
            let mut query_builder = QueryBuilder::<DB>::new("UPDATE t SET ");
            let _ = Table::push_assignments(
                &mut query_builder,
                queryweave::__private::model_data::<Model, _>(vec![
                    FieldValue {
                        field: "number",
                        value: Value::Text(None),
                    },
                    FieldValue {
                        field: "delivery_date",
                        value: Value::DateTimeUtc(None),
                    },
                ]),
            );
            query_builder.sql().to_string()
        }

        assert_eq!(
            assignments_sql::<Postgres>(),
            "UPDATE t SET number = $1, delivered_at = $2"
        );
        assert_eq!(
            assignments_sql::<MySql>(),
            "UPDATE t SET number = ?, delivered_at = ?"
        );
        assert_eq!(
            assignments_sql::<Sqlite>(),
            "UPDATE t SET number = ?, delivered_at = ?"
        );
    }

    // A renamed field is filtered, sorted and written by its column.
    #[test]
    fn a_renamed_field_is_written_as_its_column() {
        assert_eq!(
            conditions_sql::<Postgres>(vec![condition(
                "delivery_date",
                FilterOperator::GreaterThan
            )]),
            "delivered_at > $1 "
        );
        assert_eq!(
            order_by_sql::<Postgres>(vec![SortKey {
                field: "delivery_date",
                direction: SortDirectionModel::Descending,
            }]),
            "ORDER BY delivered_at DESC "
        );
    }

    // The operator selects the SQL comparison; the `LIKE` ones name the
    // escape character of their pattern.
    #[rstest]
    #[case(FilterOperator::LessThan, "value < $1 ")]
    #[case(FilterOperator::GreaterThan, "value > $1 ")]
    #[case(FilterOperator::Equal, "value = $1 ")]
    #[case(FilterOperator::StartsWith, "value LIKE $1 ESCAPE '!' ")]
    #[case(FilterOperator::Contains, "value LIKE $1 ESCAPE '!' ")]
    #[case(FilterOperator::EndsWith, "value LIKE $1 ESCAPE '!' ")]
    fn the_operator_selects_the_sql_comparison(
        #[case] operator: FilterOperator,
        #[case] expected_sql: &str,
    ) {
        assert_eq!(
            conditions_sql::<Postgres>(vec![condition("value", operator)]),
            expected_sql
        );
    }

    // The column is written verbatim, even when its name merely contains a
    // filter suffix (the old suffix-stripping turned `is_closed` into
    // `islosed`).
    #[rstest]
    #[case("is_closed")]
    #[case("user_count")]
    #[case("is_sweet")]
    #[case("is_equal_to")]
    #[case("name_c")]
    fn the_column_is_written_verbatim(#[case] column: &'static str) {
        assert_eq!(
            conditions_sql::<Postgres>(vec![condition(column, FilterOperator::Equal)]),
            format!("{column} = $1 ")
        );
    }

    // Conditions are joined by the separator, and only between them.
    #[test]
    fn conditions_are_joined_by_the_separator() {
        assert_eq!(
            conditions_sql::<Postgres>(vec![
                condition("number", FilterOperator::Equal),
                condition("client", FilterOperator::Equal),
            ]),
            "number = $1 AND client = $2 "
        );
        assert_eq!(conditions_sql::<Postgres>(Vec::new()), "");
    }

    // A non-text value is bound as it is, whatever the operator.
    #[test]
    fn a_non_text_value_is_bound_with_its_own_type() {
        let sql = conditions_sql::<Postgres>(vec![FilterCondition {
            field: "value",
            operator: FilterOperator::LessThan,
            value: Value::F64(Some(9.5)),
        }]);

        assert_eq!(sql, "value < $1 ");
    }

    // The `LIKE` operators bind the value wrapped in `%`; the comparisons bind
    // the typed value itself, so they have no pattern. The pattern is a bound
    // parameter and never appears in the rendered SQL, so it is asserted
    // through the extracted helper.
    #[rstest]
    #[case(FilterOperator::StartsWith, Some("needle%"))]
    #[case(FilterOperator::Contains, Some("%needle%"))]
    #[case(FilterOperator::EndsWith, Some("%needle"))]
    #[case(FilterOperator::Equal, None)]
    #[case(FilterOperator::LessThan, None)]
    #[case(FilterOperator::GreaterThan, None)]
    fn the_like_pattern_matches_the_operator(
        #[case] operator: FilterOperator,
        #[case] expected_pattern: Option<&str>,
    ) {
        assert_eq!(
            like_pattern(operator, "needle").as_deref(),
            expected_pattern
        );
    }

    // `LIKE` metacharacters (and the escape character itself) are escaped
    // with `!`, so they match literally; a backslash is an ordinary character.
    #[rstest]
    #[case("%", "!%")]
    #[case("_", "!_")]
    #[case("!", "!!")]
    #[case(r"\", r"\")]
    #[case(r"50%_off!\now", r"50!%!_off!!\now")]
    #[case("plain", "plain")]
    fn metacharacters_are_escaped_with_the_escape_character(
        #[case] value: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(escape_like(value), expected);
    }

    // Only the wildcards the operator adds stay live.
    #[test]
    fn the_pattern_wraps_the_escaped_text() {
        assert_eq!(
            like_pattern(FilterOperator::Contains, "50%").as_deref(),
            Some("%50!%%")
        );
    }

    // Assignments are comma separated, and more can follow on the returned
    // builder.
    #[test]
    fn assignments_are_comma_separated_and_extendable() {
        let mut query_builder = QueryBuilder::<Postgres>::new("UPDATE orders SET ");
        let mut separated = Table::push_assignments(
            &mut query_builder,
            queryweave::__private::model_data::<Model, _>(vec![
                FieldValue {
                    field: "number",
                    value: Value::Text(None),
                },
                FieldValue {
                    field: "capacity",
                    value: Value::I32(Some(3)),
                },
            ]),
        );
        separated
            .push("last_modified_at = ")
            .push_bind_unseparated(1_i64);

        assert_eq!(
            query_builder.sql(),
            "UPDATE orders SET number = $1, capacity = $2, last_modified_at = $3"
        );
    }

    // Sort keys are written in order, each with its direction, comma
    // separated only between them.
    #[test]
    fn order_by_lists_the_sort_keys_in_order() {
        assert_eq!(
            order_by_sql::<Postgres>(vec![
                SortKey {
                    field: "number",
                    direction: SortDirectionModel::Ascending,
                },
                SortKey {
                    field: "value",
                    direction: SortDirectionModel::Descending,
                },
            ]),
            "ORDER BY number ASC, value DESC "
        );
    }

    // Without sort keys there is no `ORDER BY` at all.
    #[test]
    fn order_by_is_omitted_without_sort_keys() {
        assert_eq!(order_by_sql::<Postgres>(Vec::new()), "");
    }

    // A page needs both its number and its size.
    #[rstest]
    #[case(Some(1), Some(10), "LIMIT $1 OFFSET $2")]
    #[case(None, Some(10), "")]
    #[case(Some(1), None, "")]
    #[case(None, None, "")]
    fn a_page_needs_both_its_number_and_size(
        #[case] page: Option<u32>,
        #[case] page_size: Option<u32>,
        #[case] expected_sql: &str,
    ) {
        assert_eq!(page_sql::<Postgres>(page, page_size), expected_sql);
    }

    // The offset is `page * page_size`, saturated rather than overflowing.
    #[rstest]
    #[case(0, 10, 0)]
    #[case(3, 10, 30)]
    #[case(u32::MAX, 1, 4_294_967_295)]
    #[case(u32::MAX, u32::MAX, i64::MAX)]
    fn the_offset_saturates(#[case] page: u32, #[case] page_size: u32, #[case] expected: i64) {
        assert_eq!(offset(page, page_size), expected);
    }
}
