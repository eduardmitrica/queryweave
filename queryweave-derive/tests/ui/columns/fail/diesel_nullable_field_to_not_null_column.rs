use queryweave_derive::{Columns, MergePatch, MergePatchNullableFields};

mod schema {
    diesel::table! {
        orders (id) {
            id -> Integer,
            client -> Text,
        }
    }
}

use schema::orders;

#[derive(MergePatch)]
#[merge_patch(patch = MergePatchClientModel)]
struct ClientModel {
    client: Option<String>,
}

#[derive(MergePatchNullableFields)]
struct MergePatchClientModel {
    client: queryweave::Patch<String>,
}

// The domain's `client` may be `None`, its column is `NOT NULL`: writing it
// fails to build where the changeset is made (filtering on it would not).
#[allow(dead_code)]
#[derive(diesel::Queryable, diesel::Selectable, Columns)]
#[diesel(table_name = orders)]
#[columns(model = ClientModel)]
struct Order {
    #[columns(skip)]
    id: i32,
    client: String,
}

fn main() {
    let client = ClientModel { client: None };
    let _ = Order::diesel_changeset(client.changed_values(&queryweave::ChangeSet::default()));
}
