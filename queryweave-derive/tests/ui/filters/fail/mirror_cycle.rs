// Every chain of mirrors must end at a root model: two structs mirroring each
// other have none, so their `Filterable::Root` cannot be resolved.
mod api {
    use queryweave_derive::Filters;

    #[derive(Debug, serde::Serialize, Filters)]
    #[filters(mirrors = super::grpc::UserMessage)]
    pub struct UserDto {
        #[filter(sw)]
        pub full_name: String,
    }
}

mod grpc {
    use queryweave_derive::Filters;

    #[derive(Debug, serde::Serialize, Filters)]
    #[filters(mirrors = super::api::UserDto)]
    pub struct UserMessage {
        #[filter(sw)]
        pub full_name: String,
    }
}

fn main() {}
