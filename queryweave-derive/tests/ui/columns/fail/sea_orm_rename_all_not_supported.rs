pub struct OrderModel {
    pub delivery_date: i64,
}

// SeaORM's `rename_all` would rename every column in a way the derive does
// not model.
mod order {
    use queryweave_derive::Columns;
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Columns)]
    #[sea_orm(table_name = "orders", rename_all = "camelCase")]
    #[columns(model = super::OrderModel)]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub delivery_date: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

fn main() {}
