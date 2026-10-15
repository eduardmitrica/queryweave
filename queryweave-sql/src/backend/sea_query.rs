//! sea-query's statements as queries the SQL is written into.
//!
//! A `SelectStatement`, `UpdateStatement` or `DeleteStatement` (`sea-orm`'s,
//! through `QueryTrait::query`), rendered for Postgres, MySQL/MariaDB or SQLite
//! by sea-query.
//!
//! The data becomes sea-query expressions rather than SQL text: conditions one
//! `Condition` (`all` for `AND`, `any` for `OR`) added with `cond_where`,
//! which writes the `WHERE` itself; sort keys `order_by`; a page `limit` and
//! `offset`; assignments `value`. sea-query binds every value and quotes the
//! column names.
//!
//! Public as `queryweave_sql::sea_query`: [`check_column_types`], which checks
//! a `SeaORM` entity's column types against a model's value types.

mod check;

pub use check::{ColumnTypeMismatch, MismatchReason, accepted_kinds, check_column_types};

#[cfg(feature = "merge-patch")]
use crate::AssignmentsTarget;
use crate::sql::{LIKE_ESCAPE, like_pattern, page_offset};
use crate::{Columns, ConditionsTarget, Junction, OrderByTarget, PageTarget};
use queryweave::{
    Conditions, FilterCondition, FilterOperator, SortDirectionModel, SortKey, SortKeys, Value,
};
#[cfg(feature = "merge-patch")]
use queryweave::{FieldValue, FieldValues};
#[cfg(feature = "merge-patch")]
use sea_query::UpdateStatement;
use sea_query::{
    BinOper, Condition, DeleteStatement, Expr, ExprTrait, LikeExpr, Order, SelectStatement,
};

use super::column;

/// The sea-query value of a domain raw value, with its own SQL type (an absent
/// one a `NULL` of that type).
fn sea_value(value: Value) -> sea_query::Value {
    match value {
        Value::Bool(value) => value.into(),
        Value::I16(value) => value.into(),
        Value::I32(value) => value.into(),
        Value::I64(value) => value.into(),
        Value::F32(value) => value.into(),
        Value::F64(value) => value.into(),
        Value::Text(value) => value.into(),
        Value::DateTimeUtc(value) => value.into(),
        Value::NaiveDateTime(value) => value.into(),
        Value::NaiveDate(value) => value.into(),
        Value::NaiveTime(value) => value.into(),
    }
}

/// `column operator value`, the value bound as it is.
fn comparison(column: Expr, operator: FilterOperator, value: Value) -> Expr {
    let value = sea_value(value);
    match operator {
        FilterOperator::LessThan => column.lt(value),
        FilterOperator::Equal => column.eq(value),
        FilterOperator::GreaterThan => column.gt(value),
        FilterOperator::StartsWith | FilterOperator::Contains | FilterOperator::EndsWith => {
            column.binary(BinOper::Like, value)
        },
    }
}

/// The expression for a condition, against the column `C` stores its domain
/// field in.
///
/// Like the sqlx backend: comparisons bind the value with its own SQL type;
/// the `LIKE` operators bind an escaped `%` pattern of the text and name the
/// escape character.
fn condition_expr<C: Columns + ?Sized>(condition: FilterCondition) -> Expr {
    let FilterCondition {
        field,
        operator,
        value,
    } = condition;
    let column = Expr::col(column::<C>(field));

    // The `Text` filter kind guarantees the `LIKE` operators only ever carry
    // text; anything else is bound as it is.
    match value {
        Value::Text(Some(text)) => match like_pattern(operator, &text) {
            Some(pattern) => column.like(LikeExpr::new(pattern).escape(LIKE_ESCAPE)),
            None => comparison(column, operator, Value::Text(Some(text))),
        },
        value => comparison(column, operator, value),
    }
}

/// The conditions as one sea-query condition joined by `junction`; `None`
/// when there are none, so no `WHERE` is written.
fn condition<C: Columns + ?Sized, M>(
    conditions: Conditions<M>,
    junction: Junction,
) -> Option<Condition> {
    if conditions.is_empty() {
        return None;
    }

    let joined = match junction {
        Junction::And => Condition::all(),
        Junction::Or => Condition::any(),
    };
    Some(conditions.into_iter().fold(joined, |joined, condition| {
        joined.add(condition_expr::<C>(condition))
    }))
}

/// `ConditionsTarget` for the sea-query statements taking a `WHERE` clause.
macro_rules! conditions_target {
    ($($statement:ty),+ $(,)?) => {$(
        impl ConditionsTarget for $statement {
            /// Nothing: `cond_where` writes the `WHERE` itself.
            fn write_where(&mut self) {}

            fn write_conditions<C: Columns + ?Sized, M>(
                &mut self,
                conditions: Conditions<M>,
                junction: Junction,
            ) {
                if let Some(condition) = condition::<C, M>(conditions, junction) {
                    self.cond_where(condition);
                }
            }
        }
    )+};
}

#[cfg(feature = "merge-patch")]
conditions_target!(SelectStatement, UpdateStatement, DeleteStatement);
#[cfg(not(feature = "merge-patch"))]
conditions_target!(SelectStatement, DeleteStatement);

impl OrderByTarget for SelectStatement {
    fn write_order_by<C: Columns + ?Sized, M>(&mut self, sort_keys: SortKeys<M>) {
        for SortKey { field, direction } in sort_keys {
            let order = match direction {
                SortDirectionModel::Ascending => Order::Asc,
                SortDirectionModel::Descending => Order::Desc,
            };
            self.order_by(column::<C>(field), order);
        }
    }
}

impl PageTarget for SelectStatement {
    fn write_page(&mut self, page: Option<u32>, page_size: Option<u32>) {
        let (Some(page), Some(page_size)) = (page, page_size) else {
            return;
        };

        // `page_offset` saturates at `i64::MAX` (every database's signed row
        // count), so the offset is never negative.
        self.limit(u64::from(page_size))
            .offset(page_offset(page, page_size).unsigned_abs());
    }
}

#[cfg(feature = "merge-patch")]
impl AssignmentsTarget for UpdateStatement {
    type Assignments<'target> = &'target mut Self;

    fn write_assignments<C: Columns + ?Sized, M>(
        &mut self,
        assignments: FieldValues<M>,
    ) -> Self::Assignments<'_> {
        for FieldValue { field, value } in assignments {
            self.value(column::<C>(field), sea_value(value));
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{Columns, Junction, push_page};
    use pretty_assertions::assert_eq;
    #[cfg(feature = "merge-patch")]
    use queryweave::FieldValue;
    use queryweave::{
        FilterCondition, FilterOperator, PersistedFields, SortDirectionModel, SortKey, Value,
    };
    use rstest::rstest;
    use sea_query::{
        MysqlQueryBuilder, PostgresQueryBuilder, Query, QueryBuilder, SelectStatement,
        SqliteQueryBuilder, Values,
    };

    /// Each field stored in the column of its name, except `delivery_date`,
    /// stored in `delivered_at`.
    struct Table;

    impl Columns for Table {
        const NAME: &'static str = "Table";
        const COLUMNS: &'static [(&'static str, &'static str)] = &[
            ("number", "number"),
            ("client", "client"),
            ("value", "value"),
            ("delivery_date", "delivered_at"),
        ];
    }

    /// A model whose every field has a column on [`Table`].
    struct Model;

    impl PersistedFields for Model {
        const FIELD_NAMES: &'static [&'static str] =
            &["number", "client", "value", "delivery_date"];
    }

    fn condition(field: &'static str, operator: FilterOperator) -> FilterCondition {
        FilterCondition {
            field,
            operator,
            value: Value::Text(Some(String::from("needle"))),
        }
    }

    fn select() -> SelectStatement {
        Query::select().column("id").from("orders").to_owned()
    }

    fn where_sql(conditions: Vec<FilterCondition>, junction: Junction) -> (String, Values) {
        let mut statement = select();
        Table::push_conditions(
            &mut statement,
            queryweave::__private::model_data::<Model, _>(conditions),
            junction,
        );
        statement.build(PostgresQueryBuilder)
    }

    /// Conditions, sorting and a page, as one select.
    fn select_sql(builder: &impl QueryBuilder) -> String {
        let mut statement = select();
        Table::push_conditions(
            &mut statement,
            queryweave::__private::model_data::<Model, _>(vec![
                condition("number", FilterOperator::Equal),
                condition("client", FilterOperator::StartsWith),
            ]),
            Junction::And,
        );
        Table::push_order_by(
            &mut statement,
            queryweave::__private::model_data::<Model, _>(vec![SortKey {
                field: "delivery_date",
                direction: SortDirectionModel::Descending,
            }]),
        );
        push_page(&mut statement, Some(2), Some(10));
        statement.build_any(builder).0
    }

    // Every database gets its own placeholders and identifier quotes.
    #[test]
    fn every_database_gets_its_own_placeholders_and_quotes() {
        assert_eq!(
            select_sql(&PostgresQueryBuilder),
            r#"SELECT "id" FROM "orders" WHERE "number" = $1 AND "client" LIKE $2 ESCAPE '!' ORDER BY "delivered_at" DESC LIMIT $3 OFFSET $4"#
        );
        assert_eq!(
            select_sql(&MysqlQueryBuilder),
            "SELECT `id` FROM `orders` WHERE `number` = ? AND `client` LIKE ? ESCAPE '!' ORDER BY `delivered_at` DESC LIMIT ? OFFSET ?"
        );
        assert_eq!(
            select_sql(&SqliteQueryBuilder),
            r#"SELECT "id" FROM "orders" WHERE "number" = ? AND "client" LIKE ? ESCAPE '!' ORDER BY "delivered_at" DESC LIMIT ? OFFSET ?"#
        );
    }

    // The values are bound: the comparison's as it is, the `LIKE` one as its
    // escaped pattern, the page as its size and offset.
    #[test]
    fn the_values_are_bound() {
        let mut statement = select();
        Table::push_conditions(
            &mut statement,
            queryweave::__private::model_data::<Model, _>(vec![
                FilterCondition {
                    field: "value",
                    operator: FilterOperator::LessThan,
                    value: Value::F64(Some(9.5)),
                },
                FilterCondition {
                    field: "client",
                    operator: FilterOperator::Contains,
                    value: Value::Text(Some(String::from("50%"))),
                },
            ]),
            Junction::And,
        );
        push_page(&mut statement, Some(3), Some(10));

        let (_, values) = statement.build(PostgresQueryBuilder);
        assert_eq!(
            values,
            Values(vec![
                9.5_f64.into(),
                String::from("%50!%%").into(),
                10_u64.into(),
                30_u64.into(),
            ])
        );
    }

    // Conditions are joined by the junction, and without any there is no
    // `WHERE` at all.
    #[rstest]
    #[case(
        Junction::And,
        r#"SELECT "id" FROM "orders" WHERE "number" = $1 AND "client" = $2"#
    )]
    #[case(
        Junction::Or,
        r#"SELECT "id" FROM "orders" WHERE "number" = $1 OR "client" = $2"#
    )]
    fn conditions_are_joined_by_the_junction(#[case] junction: Junction, #[case] expected: &str) {
        let conditions = vec![
            condition("number", FilterOperator::Equal),
            condition("client", FilterOperator::Equal),
        ];

        assert_eq!(where_sql(conditions, junction).0, expected);
        assert_eq!(
            where_sql(Vec::new(), junction).0,
            r#"SELECT "id" FROM "orders""#
        );
    }

    // The operator selects the comparison; the `LIKE` ones name the escape
    // character of their pattern.
    #[rstest]
    #[case(FilterOperator::LessThan, r#""value" < $1"#)]
    #[case(FilterOperator::GreaterThan, r#""value" > $1"#)]
    #[case(FilterOperator::Equal, r#""value" = $1"#)]
    #[case(FilterOperator::StartsWith, r#""value" LIKE $1 ESCAPE '!'"#)]
    #[case(FilterOperator::Contains, r#""value" LIKE $1 ESCAPE '!'"#)]
    #[case(FilterOperator::EndsWith, r#""value" LIKE $1 ESCAPE '!'"#)]
    fn the_operator_selects_the_comparison(
        #[case] operator: FilterOperator,
        #[case] expected: &str,
    ) {
        let (sql, _) = where_sql(vec![condition("value", operator)], Junction::And);

        assert_eq!(
            sql,
            format!(r#"SELECT "id" FROM "orders" WHERE {expected}"#)
        );
    }

    // A renamed field is filtered and sorted by its column.
    #[test]
    fn a_renamed_field_is_written_as_its_column() {
        let (sql, _) = where_sql(
            vec![condition("delivery_date", FilterOperator::GreaterThan)],
            Junction::And,
        );
        assert_eq!(
            sql,
            r#"SELECT "id" FROM "orders" WHERE "delivered_at" > $1"#
        );
    }

    // Without sort keys or a whole page there is no `ORDER BY` or `LIMIT`.
    #[rstest]
    #[case(None, Some(10))]
    #[case(Some(1), None)]
    fn nothing_is_written_without_sort_keys_or_a_whole_page(
        #[case] page: Option<u32>,
        #[case] page_size: Option<u32>,
    ) {
        let mut statement = select();
        Table::push_order_by(
            &mut statement,
            queryweave::__private::model_data::<Model, _>(Vec::new()),
        );
        push_page(&mut statement, page, page_size);

        assert_eq!(
            statement.to_string(PostgresQueryBuilder),
            r#"SELECT "id" FROM "orders""#
        );
    }

    // Assignments set each column, and more can follow on the returned
    // statement.
    #[cfg(feature = "merge-patch")]
    #[test]
    fn assignments_set_each_column_and_are_extendable() {
        let mut statement = Query::update().table("orders").to_owned();
        Table::push_assignments(
            &mut statement,
            queryweave::__private::model_data::<Model, _>(vec![
                FieldValue {
                    field: "number",
                    value: Value::Text(None),
                },
                FieldValue {
                    field: "delivery_date",
                    value: Value::I32(Some(3)),
                },
            ]),
        )
        .value("last_modified_at", 1_i64);

        assert_eq!(
            statement.build(PostgresQueryBuilder).0,
            r#"UPDATE "orders" SET "number" = $1, "delivered_at" = $2, "last_modified_at" = $3"#
        );
    }
}
