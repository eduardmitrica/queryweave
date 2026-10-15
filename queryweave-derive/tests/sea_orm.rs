//! The generated code writing into `sea-orm` queries: an entity deriving both
//! `DeriveEntityModel` and `Columns`, its `#[sea_orm(column_name)]` honoured,
//! and the query wrappers and `push_assignments` writing into its `Select` /
//! `UpdateMany` through `QueryTrait::query`.

use chrono::{DateTime, Utc};
use pretty_assertions::assert_eq;
use queryweave::{ChangeSet, ChangedFields, SortDirectionModel};
use queryweave_derive::{
    AppendConditions, AppendToQuery, Filters, MergePatch, MergePatchNullableFields,
};
use sea_orm::sea_query::SelectStatement;
use sea_orm::{DbBackend, EntityTrait, QueryTrait};

#[allow(dead_code)]
#[derive(Filters)]
struct OrderModel {
    #[filter(lt, gt)]
    #[sortable]
    delivery_date: DateTime<Utc>,

    #[filter(sw, eq)]
    client: String,
}

/// The changes written back: a merge patch model of its own, its fields
/// stored by the same entity.
#[derive(Clone, MergePatch)]
#[merge_patch(patch = MergePatchClientModel)]
struct ClientModel {
    client: String,
    delivery_date: DateTime<Utc>,
}

#[derive(Default, MergePatchNullableFields)]
struct MergePatchClientModel {
    client: Option<String>,
    delivery_date: Option<DateTime<Utc>>,
}

/// The `sea-orm` entity: `delivery_date` is stored in the `delivered_on` column
/// (through both `#[domain_field]` and `#[sea_orm(column_name)]`).
mod order {
    use queryweave_derive::Columns;
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Columns)]
    #[sea_orm(table_name = "orders")]
    #[columns(model = super::OrderModel)]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[domain_field(delivery_date)]
        #[sea_orm(column_name = "delivered_on")]
        pub delivered_at: ChronoDateTimeUtc,
        pub client: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

#[derive(Debug, AppendConditions)]
#[append_conditions(columns = order::Model)]
struct OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel);

#[derive(Debug, AppendToQuery)]
#[append_to_query(columns = order::Model, filters = OrdersFilterOptionsModelWrapper)]
struct OrdersQueryModelWrapper(OrdersQueryModel);

type NoGuard = Option<fn(&mut SelectStatement, &OrdersQueryModelWrapper)>;

fn query() -> OrdersQueryModel {
    OrdersQueryModel {
        page: Some(2),
        page_size: Some(10),
        sort: Some(vec![OrderSortDetailsModel {
            field: OrderSortFieldModel::DeliveryDate,
            order: SortDirectionModel::Descending,
        }]),
        and: OrdersFilterOptionsModel {
            client_sw: Some(String::from("ab")),
            ..Default::default()
        },
    }
}

// Filters, sorting (by the renamed column) and the page, in a SeaORM select.
#[test]
fn a_query_writes_into_a_sea_orm_select() {
    let mut select = order::Entity::find();
    OrdersQueryModelWrapper(query()).append_to_query(select.query(), NoGuard::None);

    assert_eq!(
        select.build(DbBackend::Postgres).sql,
        r#"SELECT "orders"."id", "orders"."delivered_on", "orders"."client" FROM "orders" WHERE "client" LIKE $1 ESCAPE '!' ORDER BY "delivered_on" DESC LIMIT $2 OFFSET $3"#
    );
}

// The privileged guard adds its own conditions to the statement's `WHERE`.
#[test]
fn the_privileged_guard_adds_to_the_where_clause() {
    use sea_orm::sea_query::{Expr, ExprTrait};

    let mut select = order::Entity::find();
    OrdersQueryModelWrapper(query()).append_to_query(
        select.query(),
        Some(
            |statement: &mut SelectStatement, _: &OrdersQueryModelWrapper| {
                statement.and_where(Expr::col("id").gt(0));
            },
        ),
    );

    assert_eq!(
        select.build(DbBackend::MySql).sql,
        "SELECT `orders`.`id`, `orders`.`delivered_on`, `orders`.`client` FROM `orders` WHERE `client` LIKE ? ESCAPE '!' AND `id` > ? ORDER BY `delivered_on` DESC LIMIT ? OFFSET ?"
    );
}

// A merge patch's changed values set the columns of a SeaORM update.
#[test]
fn changed_values_write_into_a_sea_orm_update() {
    let before = ClientModel {
        client: String::from("acme"),
        delivery_date: DateTime::<Utc>::UNIX_EPOCH,
    };
    let mut client = before.clone();
    client.merge(MergePatchClientModel {
        client: Some(String::from("globex")),
        ..Default::default()
    });
    let changed: ChangeSet = client.modified_fields(&before);

    let mut update = order::Entity::update_many();
    order::Model::push_assignments(update.query(), client.changed_values(&changed));

    assert_eq!(
        update.build(DbBackend::Sqlite).sql,
        r#"UPDATE "orders" SET "client" = ?"#
    );
}

/// An entity storing the delivery date as text: the domain and the schema
/// disagree on its type.
mod text_dated_order {
    use queryweave_derive::Columns;
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Columns)]
    #[sea_orm(table_name = "orders")]
    #[columns(model = super::OrderModel)]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[domain_field(delivery_date)]
        #[sea_orm(column_name = "delivered_on")]
        pub delivered_at: String,
        pub client: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// `(name, type, nullable)` of each column of the `SeaORM` entity `E`.
fn columns<E: EntityTrait>() -> Vec<(&'static str, sea_orm::ColumnType, bool)> {
    use sea_orm::{ColumnTrait, IdenStatic, Iterable};

    E::Column::iter()
        .map(|column| {
            let def = column.def();
            (
                column.as_str(),
                def.get_column_type().clone(),
                def.is_null(),
            )
        })
        .collect()
}

// The entity's column types fit the filters' and the merge patch's values.
#[test]
fn the_entity_column_types_fit_the_model_values() {
    use queryweave_sql::sea_query::check_column_types;

    assert_eq!(
        check_column_types::<order::Model, OrdersFilterOptionsModel>(columns::<order::Entity>()),
        Ok(())
    );
    assert_eq!(
        check_column_types::<order::Model, ClientModel>(columns::<order::Entity>()),
        Ok(())
    );
}

// A column of another type is reported, naming the field and the column.
#[test]
fn a_column_of_another_type_is_reported() {
    use queryweave_sql::sea_query::check_column_types;

    let mismatch =
        check_column_types::<text_dated_order::Model, OrdersFilterOptionsModel>(columns::<
            text_dated_order::Entity,
        >())
        .err()
        .map(|mismatch| mismatch.to_string());

    assert_eq!(
        mismatch.as_deref(),
        Some(
            "the value of domain field `delivery_date` (`DateTime<Utc>`) does not fit the column `delivered_on` of `Model`, which takes `String`: make the field's type and the column's type in the entity agree"
        )
    );
}
