/// What the code generated for a `MergePatchNullableFields` merge patch model links to: the runtime crate.
#[derive(Debug, PartialEq, Eq)]
pub struct MappingPaths {
    /// The path of the core runtime crate.
    pub runtime: syn::Path,
}

/// The core runtime crate: the one given by `#[queryweave(crate = path)]`, or
/// the conventions' `C::core_crate()`.
#[must_use]
pub fn generate_paths<C: crate::conventions::Conventions>(
    runtime_crate: Option<syn::Path>,
) -> MappingPaths {
    MappingPaths {
        runtime: runtime_crate.unwrap_or_else(C::core_crate),
    }
}
