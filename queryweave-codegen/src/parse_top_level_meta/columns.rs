/// The options of `#[columns(...)]`.
const COLUMNS_OPTIONS: [(&str, crate::attributes::OptionKind); 1] =
    [("model", crate::attributes::OptionKind::Path)];

/// The struct attributes of a `Columns` entity.
#[derive(Debug, PartialEq, Eq)]
pub struct TopLevelMeta {
    /// `#[columns(model = path)]`: the domain model whose fields the entity's
    /// columns store.
    pub columns_for: syn::Path,
    /// `#[queryweave(crate = path)]`: where the runtime crate the generated
    /// code names lives, overriding the conventions' default.
    pub runtime_crate: Option<syn::Path>,
}

impl crate::steps::TopLevelMeta for TopLevelMeta {
    fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self> {
        let columns_for = crate::attributes::required_path_option(
            attributes,
            "columns",
            "model",
            &COLUMNS_OPTIONS,
            "Name the domain model whose fields this entity's columns store: #[columns(model = path)]",
        );
        let runtime_crate = crate::attributes::crate_option(attributes);

        // Report every malformed attribute at once.
        let (columns_for, runtime_crate) = crate::errors::both(columns_for, runtime_crate)?;

        Ok(Self {
            columns_for,
            runtime_crate,
        })
    }
}
