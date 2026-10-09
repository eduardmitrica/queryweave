/// The sort fields, as index-aligned lists.
#[derive(Debug, PartialEq, Eq)]
pub struct Sort {
    /// The variant of the generated sort-field enum for each sortable field.
    pub variants: Vec<syn::Ident>,
    /// Each sortable field's own name (same index in `variants`).
    pub fields: Vec<syn::Ident>,
    /// The sort variant each sortable field (same index in `variants`) has on
    /// the equivalent type.
    pub equivalent_variants: Vec<syn::Ident>,
}

/// The intermediary representation that will be turned back into Rust code.
#[derive(Debug, PartialEq, Eq)]
pub struct IntermediaryRepresentation {
    pub idents: crate::conventions::FiltersNames,
    pub field_idents: Vec<syn::Ident>,
    /// The ident each filter field (same index in `field_idents`) has on the
    /// equivalent type.
    pub equivalent_field_idents: Vec<syn::Ident>,
    /// The value type each filter field (same index in `field_idents`) is
    /// derived from: the struct field's type, less one outer `Option`.
    pub field_value_types: Vec<syn::Type>,
    /// The column each filter field (same index in `field_idents`) compares against.
    pub field_columns: Vec<String>,
    /// The operator each filter field (same index in `field_idents`) compares with.
    pub field_operators: Vec<crate::analyze::filters::FilterOperator>,
    pub sort: Option<Sort>,
}

impl IntermediaryRepresentation {
    #[must_use]
    fn from_model_with_idents(
        model: crate::analyze::filters::Model,
        idents: crate::conventions::FiltersNames,
    ) -> Self {
        Self {
            idents,
            field_idents: model.field_idents,
            equivalent_field_idents: model.equivalent_field_idents,
            field_value_types: model.field_value_types,
            field_columns: model.field_columns,
            field_operators: model.field_operators,
            sort: if model.sort_fields.is_empty() || model.snake_cased_sort_fields.is_empty() {
                None
            } else {
                Some(Sort {
                    variants: model.sort_fields,
                    fields: model.snake_cased_sort_fields,
                    equivalent_variants: model.equivalent_sort_fields,
                })
            },
        }
    }
}

/// Lower the model of a `Filters` derive (domain or API), naming the generated
/// types with `idents`.
#[must_use]
pub fn lower(
    model: crate::analyze::filters::Model,
    idents: crate::conventions::FiltersNames,
) -> IntermediaryRepresentation {
    IntermediaryRepresentation::from_model_with_idents(model, idents)
}

#[cfg(test)]
mod tests {

    fn idents() -> crate::conventions::FiltersNames {
        crate::conventions::FiltersNames {
            query: quote::format_ident!("UsersQueryModel"),
            filters: quote::format_ident!("UsersFilterOptionsModel"),
            sort_field: quote::format_ident!("UserSortFieldModel"),
            sort_details: quote::format_ident!("UserSortDetailsModel"),
            input: quote::format_ident!("UserModel"),
            page: quote::format_ident!("UsersPageModel"),
            query_params: quote::format_ident!("UsersQueryParams"),
        }
    }

    fn model_with_sort_fields(
        sort_fields: Vec<syn::Ident>,
        snake_cased_sort_fields: Vec<syn::Ident>,
    ) -> crate::analyze::filters::Model {
        crate::analyze::filters::Model {
            field_idents: vec![quote::format_ident!("username_sw")],
            equivalent_field_idents: vec![quote::format_ident!("username_sw")],
            field_value_types: vec![syn::parse_str::<syn::Type>("String").unwrap()],
            field_columns: vec![String::from("username")],
            field_operators: vec![crate::analyze::filters::FilterOperator::StartsWith],
            equivalent_sort_fields: sort_fields.clone(),
            sort_fields,
            snake_cased_sort_fields,
        }
    }

    // A model with nothing sortable lowers to no sort details at all.
    #[test]
    fn test_lowering_without_sort_fields_yields_no_sort() {
        let intermediary_representation =
            super::lower(model_with_sort_fields(Vec::new(), Vec::new()), idents());

        assert!(intermediary_representation.sort.is_none());
    }

    // Both halves are needed: a model carrying only one of them is incomplete
    // and must not produce a half-populated `Sort`.
    #[test]
    fn test_lowering_with_only_one_sort_half_yields_no_sort() {
        let only_pascal_cased = super::lower(
            model_with_sort_fields(vec![quote::format_ident!("Username")], Vec::new()),
            idents(),
        );
        assert!(only_pascal_cased.sort.is_none());

        let only_snake_cased = super::lower(
            model_with_sort_fields(Vec::new(), vec![quote::format_ident!("username")]),
            idents(),
        );
        assert!(only_snake_cased.sort.is_none());
    }

    #[test]
    fn test_lowering_into_ir() {
        let idents = crate::conventions::FiltersNames {
            query: quote::format_ident!("UsersQueryModel"),
            filters: quote::format_ident!("UsersFilterOptionsModel"),
            sort_field: quote::format_ident!("UserSortFieldModel"),
            sort_details: quote::format_ident!("UserSortDetailsModel"),
            input: quote::format_ident!("UserModel"),
            page: quote::format_ident!("UsersPageModel"),
            query_params: quote::format_ident!("UsersQueryParams"),
        };
        // A renamed field (`#[equivalent_field(user_name)]`): its equivalent
        // names are carried through alongside its own.
        let model = crate::analyze::filters::Model {
            field_idents: vec![quote::format_ident!("username_sw")],
            equivalent_field_idents: vec![quote::format_ident!("user_name_sw")],
            field_value_types: vec![syn::parse_str::<syn::Type>("String").unwrap()],
            field_columns: vec![String::from("username")],
            field_operators: vec![crate::analyze::filters::FilterOperator::StartsWith],
            sort_fields: vec![quote::format_ident!("Username")],
            snake_cased_sort_fields: vec![quote::format_ident!("username")],
            equivalent_sort_fields: vec![quote::format_ident!("UserName")],
        };

        let intermediary_representation = super::lower(model, idents.clone());
        let expected_intermediary_representation = super::IntermediaryRepresentation {
            field_idents: vec![quote::format_ident!("username_sw")],
            equivalent_field_idents: vec![quote::format_ident!("user_name_sw")],
            field_value_types: vec![syn::parse_str::<syn::Type>("String").unwrap()],
            field_columns: vec![String::from("username")],
            field_operators: vec![crate::analyze::filters::FilterOperator::StartsWith],
            idents,
            sort: Some(super::Sort {
                variants: vec![quote::format_ident!("Username")],
                fields: vec![quote::format_ident!("username")],
                equivalent_variants: vec![quote::format_ident!("UserName")],
            }),
        };
        pretty_assertions::assert_eq!(
            expected_intermediary_representation,
            intermediary_representation
        );
    }
}
