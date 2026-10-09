/// The options of `#[append_to_query(...)]` on a filter options wrapper: its
/// filters are its own, so it takes no `filters` wrapper.
const APPEND_TO_QUERY_OPTIONS: [(&str, crate::attributes::OptionKind); 1] =
    [("columns", crate::attributes::OptionKind::Path)];

/// The struct attributes of an `AppendToQueryWithSeparator` wrapper.
#[derive(Debug, PartialEq, Eq)]
pub struct TopLevelMeta {
    /// `#[append_to_query(columns = path)]`: the entity whose
    /// `queryweave_sql::Columns` store the wrapped filters' domain fields.
    pub columns: syn::Path,
    /// `#[queryweave(crate = path)]`: where the runtime crate the generated
    /// code names lives, overriding the conventions' default.
    pub runtime_crate: Option<syn::Path>,
}

impl crate::steps::TopLevelMeta for TopLevelMeta {
    fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self> {
        let columns = crate::attributes::required_path_option(
            attributes,
            "append_to_query",
            "columns",
            &APPEND_TO_QUERY_OPTIONS,
            "Name the entity (deriving Columns) that stores the wrapped filters' fields: #[append_to_query(columns = path)]",
        );
        let runtime_crate = crate::attributes::crate_option(attributes);

        // Report every malformed attribute at once.
        let (columns, runtime_crate) = crate::errors::both(columns, runtime_crate)?;

        Ok(Self {
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
            #[derive(Debug, AppendToQueryWithSeparator)]
            #[append_to_query(columns = crate::entities::order::Order)]
            pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);
        })
        .unwrap();

        pretty_assertions::assert_eq!(
            top_level_meta,
            super::TopLevelMeta {
                columns: syn::parse_str("crate::entities::order::Order").unwrap(),
                runtime_crate: None,
            }
        );
    }

    // A filter options wrapper's filters are its own: it takes no `filters`.
    #[test]
    fn test_parsing_top_level_meta_rejects_a_filters_option() {
        assert!(
            parse(quote::quote! {
                #[append_to_query(columns = crate::entities::order::Order, filters = Other)]
                pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);
            })
            .is_err()
        );
    }

    #[test]
    fn test_parsing_top_level_meta_fails_without_columns() {
        assert!(
            parse(quote::quote! {
                pub struct OrdersFilterOptionsModelWrapper(pub OrdersFilterOptionsModel);
            })
            .is_err()
        );
    }
}
