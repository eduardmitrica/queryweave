/// What an `AppendConditions` wrapper links to: the entity whose
/// columns store the wrapped filters' domain fields.
#[derive(Debug, PartialEq, Eq)]
pub struct MappingPaths {
    pub columns: syn::Path,
    /// The path of the runtime crate the generated code names: the one
    /// given by `#[queryweave(crate = path)]`, or the conventions' default.
    pub runtime: syn::Path,
}

/// The entity named by `#[append_conditions(columns = path)]`: the compiler checks it is one
/// deriving `Columns` where the generated code names it.
#[must_use]
pub fn generate_paths<C: crate::conventions::Conventions>(
    top_level_meta: crate::parse_top_level_meta::append_conditions::TopLevelMeta,
) -> MappingPaths {
    MappingPaths {
        columns: top_level_meta.columns,
        runtime: top_level_meta.runtime_crate.unwrap_or_else(C::sql_crate),
    }
}
