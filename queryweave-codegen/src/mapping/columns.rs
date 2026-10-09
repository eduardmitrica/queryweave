/// What a `Columns` entity links to: the domain model whose fields its
/// columns store.
#[derive(Debug, PartialEq, Eq)]
pub struct MappingPaths {
    pub domain_model: syn::Path,
    /// The path of the runtime crate the generated code names: the one
    /// given by `#[queryweave(crate = path)]`, or the conventions' default.
    pub runtime: syn::Path,
}

/// The domain model named by `#[columns(model = path)]`: the generated code checks
/// the mapped fields exist on it.
#[must_use]
pub fn generate_paths<C: crate::conventions::Conventions>(
    top_level_meta: crate::parse_top_level_meta::columns::TopLevelMeta,
) -> MappingPaths {
    MappingPaths {
        domain_model: top_level_meta.columns_for,
        runtime: top_level_meta.runtime_crate.unwrap_or_else(C::sql_crate),
    }
}
