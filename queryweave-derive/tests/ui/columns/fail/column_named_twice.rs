pub struct OrderModel {
    pub number: String,
}

// sqlx and SeaORM name the same column differently: the derive cannot tell
// which one the table has.
mod order {
    use queryweave_derive::Columns;
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow, DeriveEntityModel, Columns)]
    #[sea_orm(table_name = "orders")]
    #[columns(model = super::OrderModel)]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[sqlx(rename = "number")]
        #[sea_orm(column_name = "order_number")]
        pub number: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

fn main() {}
