/// Everything an `AppendToQuery` wrapper links to: the wrapper building its
/// `WHERE` clause, and the entity whose columns store the model's fields.
#[derive(Debug, PartialEq, Eq)]
pub struct MappingPaths {
    pub with_wrapper: syn::Path,
    pub columns: syn::Path,
    /// The path of the runtime crate the generated code names: the one
    /// given by `#[queryweave(crate = path)]`, or the conventions' default.
    pub runtime: syn::Path,
}

/// Check the wrapper named by `#[append_to_query(filters = path)]`, next to the
/// entity named by `#[append_to_query(columns = path)]`.
///
/// # Errors
///
/// Returns an error, at the wrapper path, when it names no struct, or one
/// whose name does not fit the conventions `C`.
pub fn generate_paths<C: crate::conventions::Conventions>(
    top_level_meta: crate::parse_top_level_meta::append_to_query::TopLevelMeta,
) -> syn::Result<MappingPaths> {
    let crate::parse_top_level_meta::append_to_query::TopLevelMeta {
        conditions_wrapper,
        columns,
        runtime_crate,
    } = top_level_meta;

    let what = "filter-options wrapper";
    C::check_wrapper_name(
        crate::names::path_name(&conditions_wrapper, what)?,
        &conditions_wrapper,
        what,
    )?;

    Ok(MappingPaths {
        with_wrapper: conditions_wrapper,
        columns,
        runtime: runtime_crate.unwrap_or_else(C::sql_crate),
    })
}

#[cfg(test)]
mod tests {
    fn top_level_meta(
        conditions_wrapper: &str,
    ) -> crate::parse_top_level_meta::append_to_query::TopLevelMeta {
        crate::parse_top_level_meta::append_to_query::TopLevelMeta {
            conditions_wrapper: syn::parse_str(conditions_wrapper).unwrap(),
            columns: syn::parse_str("crate::entities::order::Order").unwrap(),
            runtime_crate: None,
        }
    }

    #[test]
    fn test_generating_mapping_paths_succeeds_when_all_preconditions_are_met() {
        let mapping_paths = super::generate_paths::<crate::conventions::DefaultConventions>(
            top_level_meta("OrdersFilterOptionsModelWrapper"),
        )
        .unwrap();

        pretty_assertions::assert_eq!(
            mapping_paths,
            super::MappingPaths {
                with_wrapper: syn::parse_str("OrdersFilterOptionsModelWrapper").unwrap(),
                columns: syn::parse_str("crate::entities::order::Order").unwrap(),
                runtime: syn::parse_quote!(::queryweave_sql),
            }
        );
    }

    #[test]
    fn test_generating_mapping_paths_fails_when_incorrect_equivalent_model_name_is_provided() {
        assert!(
            super::generate_paths::<crate::conventions::DefaultConventions>(top_level_meta(
                "orders_filter_options_model_wrapper"
            ))
            .is_err()
        );
    }
}
