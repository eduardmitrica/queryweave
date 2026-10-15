/// The options of `#[append_to_query(...)]` on a query model wrapper.
const APPEND_TO_QUERY_OPTIONS: [(&str, crate::attributes::OptionKind); 2] = [
    ("columns", crate::attributes::OptionKind::Path),
    ("filters", crate::attributes::OptionKind::Path),
];

/// The struct attributes of an `AppendToQuery` wrapper.
#[derive(Debug, PartialEq, Eq)]
pub struct TopLevelMeta {
    /// `#[append_to_query(filters = path)]`: the wrapper building the `WHERE`
    /// clause from the query model's filters.
    pub conditions_wrapper: syn::Path,
    /// `#[append_to_query(columns = path)]`: the entity whose
    /// `queryweave_sql::Columns` store the wrapped model's domain fields.
    pub columns: syn::Path,
    /// `#[queryweave(crate = path)]`: where the runtime crate the generated
    /// code names lives, overriding the conventions' default.
    pub runtime_crate: Option<syn::Path>,
}

impl crate::steps::TopLevelMeta for TopLevelMeta {
    fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self> {
        // One attribute, read option by option: its first malformed option
        // is enough (each read checks every option), but every missing one
        // is reported.
        let append_to_query = (|| {
            const COLUMNS_HELP: &str = "Name the entity (deriving Columns) that stores the wrapped model's fields: #[append_to_query(columns = path)]";
            const FILTERS_HELP: &str = "Name the wrapper deriving AppendConditions over the filters of this query model: #[append_to_query(filters = path)]";
            let read = |key, help| {
                crate::attributes::path_option(
                    attributes,
                    "append_to_query",
                    key,
                    &APPEND_TO_QUERY_OPTIONS,
                    help,
                )
            };
            let columns = read("columns", COLUMNS_HELP)?;
            let filters = read("filters", FILTERS_HELP)?;

            let require = |path, key, help| {
                crate::attributes::require_option(path, "append_to_query", key, help)
            };
            crate::errors::both(
                require(filters, "filters", FILTERS_HELP),
                require(columns, "columns", COLUMNS_HELP),
            )
        })();
        let runtime_crate = crate::attributes::crate_option(attributes);

        // Report every malformed attribute at once.
        let ((conditions_wrapper, columns), runtime_crate) =
            crate::errors::both(append_to_query, runtime_crate)?;

        Ok(Self {
            conditions_wrapper,
            columns,
            runtime_crate,
        })
    }
}

#[cfg(test)]
mod tests {

    fn parse(item: proc_macro2::TokenStream) -> syn::Result<super::TopLevelMeta> {
        let attributes = syn::parse2::<syn::DeriveInput>(item).unwrap().attrs;

        crate::steps::TopLevelMeta::from_attributes(&attributes)
    }

    #[test]
    fn test_parsing_top_level_meta_succeeds_when_all_preconditions_are_met() {
        let top_level_meta = parse(quote::quote! {
            #[derive(Debug, AppendToQuery)]
            #[append_to_query(columns = crate::entities::order::Order, filters = OrdersFilterOptionsModelWrapper)]
            pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);
        })
        .unwrap();

        pretty_assertions::assert_eq!(
            top_level_meta,
            super::TopLevelMeta {
                conditions_wrapper: syn::parse_str("OrdersFilterOptionsModelWrapper").unwrap(),
                columns: syn::parse_str("crate::entities::order::Order").unwrap(),
                runtime_crate: None,
            }
        );
    }

    #[test]
    fn test_parsing_top_level_meta_fails_when_empty_path_is_provided() {
        assert!(
            parse(quote::quote! {
                #[append_to_query(columns = crate::entities::order::Order, filters)]
                pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);
            })
            .is_err()
        );
    }

    #[test]
    fn test_parsing_top_level_meta_fails_when_no_last_path_segment_is_provided() {
        assert!(
            parse(quote::quote! {
                #[append_to_query(columns = crate::entities::order::Order, filters = infrastructure::orders_filter_option_wrapper::)]
                pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);
            })
            .is_err()
        );
    }

    // Both attributes are required; each missing one is reported.
    #[test]
    fn test_parsing_top_level_meta_reports_every_missing_attribute() {
        let errors = parse(quote::quote! {
            pub struct OrdersQueryModelWrapper(pub OrdersQueryModel);
        })
        .unwrap_err()
        .into_iter()
        .map(|error| error.to_string())
        .collect::<Vec<_>>();

        assert_eq!(errors.len(), 2);
        assert!(errors[0].starts_with("No #[append_to_query(filters = ...)] attribute provided"));
        assert!(errors[1].starts_with("No #[append_to_query(columns = ...)] attribute provided"));
    }
}
