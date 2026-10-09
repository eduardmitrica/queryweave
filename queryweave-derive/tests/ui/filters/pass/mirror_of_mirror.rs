use queryweave::IntoRootQuery;
use queryweave_derive::Filters;

#[derive(Filters)]
pub struct UserModel {
    #[filter(sw)]
    #[sortable]
    pub full_name: String,
}

#[derive(Debug, serde::Serialize, Filters)]
#[filters(mirrors = UserModel, serde, utoipa, validator)]
pub struct UserDto {
    #[filter(sw)]
    #[sortable]
    #[equivalent_field(full_name)]
    pub name: String,
}

mod grpc {
    use queryweave_derive::Filters;

    // A mirror of the DTO, itself a mirror: its names map onto the DTO's. A
    // plain one, its sort direction converting into the DTO's.
    #[derive(Debug, Filters)]
    #[filters(mirrors = super::UserDto)]
    pub struct UserMessage {
        #[filter(sw)]
        #[sortable]
        #[equivalent_field(name)]
        pub display_name: String,
    }
}

fn main() {
    let query: UsersQueryModel = grpc::UsersQueryParams::default().into_root_query();
    let _ = query.and.full_name_sw;
}
