//! sqlx's [`QueryBuilder`] as a query the SQL is written into, for any
//! database whose driver binds the domain's raw values ([`BindValues`]):
//! Postgres, MySQL/MariaDB and SQLite.
//!
//! The clauses are appended as SQL text, the values bound with the builder's
//! placeholders (`$1` on Postgres, `?` elsewhere).

mod bind;

pub use bind::BindValues;

use super::column;
#[cfg(feature = "merge-patch")]
use crate::AssignmentsTarget;
use crate::sql::{
    LIKE_ESCAPE, like_pattern, page_offset, sql_direction, sql_junction, sql_operator,
};
use crate::{Columns, ConditionsTarget, Junction, OrderByTarget, PageTarget};
use queryweave::{Conditions, FilterCondition, SortKey, SortKeys, Value};
#[cfg(feature = "merge-patch")]
use queryweave::{FieldValue, FieldValues};
use sqlx::QueryBuilder;
#[cfg(feature = "merge-patch")]
use sqlx::query_builder::Separated;

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

impl<DB: BindValues> ConditionsTarget for QueryBuilder<'_, DB> {
    fn write_where(&mut self) {
        self.push("WHERE ");
    }

    fn write_conditions<C: Columns + ?Sized, M>(
        &mut self,
        conditions: Conditions<M>,
        junction: Junction,
    ) {
        for (index, condition) in conditions.into_iter().enumerate() {
            if index > 0 {
                self.push(format!("{} ", sql_junction(junction)));
            }
            push_condition::<C, DB>(self, condition);
        }
    }
}

impl<DB: BindValues> OrderByTarget for QueryBuilder<'_, DB> {
    fn write_order_by<C: Columns + ?Sized, M>(&mut self, sort_keys: SortKeys<M>) {
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
        self.push(format!("ORDER BY {order_by} "));
    }
}

#[cfg(feature = "merge-patch")]
impl<'args, DB: BindValues> AssignmentsTarget for QueryBuilder<'args, DB> {
    type Assignments<'target>
        = Separated<'target, 'args, DB, &'static str>
    where
        Self: 'target;

    fn write_assignments<C: Columns + ?Sized, M>(
        &mut self,
        assignments: FieldValues<M>,
    ) -> Self::Assignments<'_> {
        let mut separated = self.separated(", ");

        for FieldValue { field, value } in assignments {
            separated.push(format!("{} = ", column::<C>(field)));
            DB::push_bind_value_unseparated(&mut separated, value);
        }

        separated
    }
}

impl<DB: BindValues> PageTarget for QueryBuilder<'_, DB> {
    fn write_page(&mut self, page: Option<u32>, page_size: Option<u32>) {
        let (Some(page), Some(page_size)) = (page, page_size) else {
            return;
        };

        self.push("LIMIT ");
        DB::push_bind_value(self, Value::I64(Some(i64::from(page_size))));
        self.push(" OFFSET ");
        DB::push_bind_value(self, Value::I64(Some(page_offset(page, page_size))));
    }
}

#[cfg(test)]
mod tests {
    use super::BindValues;
    use crate::{Columns, Junction, push_page};
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
            Junction::And,
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
            Junction::And,
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

    // Conditions are joined by the junction's keyword, and only between them.
    #[rstest]
    #[case(Junction::And, "number = $1 AND client = $2 ")]
    #[case(Junction::Or, "number = $1 OR client = $2 ")]
    fn conditions_are_joined_by_the_junction(
        #[case] junction: Junction,
        #[case] expected_sql: &str,
    ) {
        let mut query_builder = QueryBuilder::<Postgres>::default();
        Table::push_conditions(
            &mut query_builder,
            queryweave::__private::model_data::<Model, _>(vec![
                condition("number", FilterOperator::Equal),
                condition("client", FilterOperator::Equal),
            ]),
            junction,
        );

        assert_eq!(query_builder.sql(), expected_sql);
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
}
