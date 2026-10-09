//! Each integration on its own (`validator` with the `serde` it needs), on a
//! sortable and an unsortable mirror, and a plain mirror naming none: each
//! builds with only its own crate.
use queryweave::IntoRootQuery;
use queryweave_derive::Filters;

#[derive(Filters)]
pub struct UserModel {
    #[filter(sw)]
    #[sortable]
    pub full_name: String,
}

macro_rules! mirrors {
    ($($module:ident: $($flag:ident),*;)*) => {
        $(
            mod $module {
                use queryweave_derive::Filters;

                #[derive(Filters)]
                #[filters(mirrors = super::UserModel $(, $flag)*)]
                pub struct UserDto {
                    #[filter(sw)]
                    #[sortable]
                    pub full_name: String,
                }

                pub mod unsortable {
                    use queryweave_derive::Filters;

                    #[derive(Filters)]
                    #[filters(mirrors = super::super::UserModel $(, $flag)*)]
                    pub struct UserDto {
                        #[filter(sw)]
                        pub full_name: String,
                    }
                }
            }
        )*
    };
}

mirrors! {
    plain: ;
    with_serde: serde;
    with_utoipa: utoipa;
    with_validator: serde, validator;
    with_schemars: schemars;
    with_garde: garde;
}

fn main() {
    // A plain mirror's direction is the core one.
    let query: UsersQueryModel = plain::UsersQueryParams {
        sort: Some(vec![plain::UserSortDetailsDto {
            field: plain::UserSortFieldDto::FullName,
            order: queryweave::SortDirectionModel::Descending,
        }]),
        ..Default::default()
    }
    .into_root_query();
    let _ = query.sort;

    let _: UsersQueryModel = plain::unsortable::UsersQueryParams::default().into_root_query();
    let _: UsersQueryModel = with_serde::UsersQueryParams::default().into_root_query();
    let _: UsersQueryModel = with_utoipa::UsersQueryParams::default().into_root_query();
    let _: UsersQueryModel = with_validator::UsersQueryParams::default().into_root_query();
    let _ = <with_validator::UsersQueryParams as validator::Validate>::validate;
    let _: UsersQueryModel = with_schemars::UsersQueryParams::default().into_root_query();
    let _: UsersQueryModel = with_garde::UsersQueryParams::default().into_root_query();
    let _ = <with_garde::UsersQueryParams as garde::Validate>::validate;
}
