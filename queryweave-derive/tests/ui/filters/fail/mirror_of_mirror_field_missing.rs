use queryweave_derive::Filters;

#[derive(Filters)]
pub struct UserModel {
    #[filter(sw)]
    pub full_name: String,
}

#[derive(Debug, serde::Serialize, Filters)]
#[filters(mirrors = UserModel)]
pub struct UserDto {
    #[filter(sw)]
    pub full_name: String,
}

// A mirror of a mirror maps onto the intermediate mirror's names: `nickname`
// is not one of `UserDto`'s filters, which is reported at the field.
mod grpc {
    use queryweave_derive::Filters;

    #[derive(Debug, serde::Serialize, Filters)]
    #[filters(mirrors = super::UserDto)]
    pub struct UserMessage {
        #[filter(sw)]
        pub nickname: String,
    }
}

fn main() {}
