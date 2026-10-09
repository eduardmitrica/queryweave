use crate::{analyze, code_gen, lower, mapping, parse, parse_top_level_meta};

/// `#[derive(MergePatch)]` on a domain model.
///
/// Its naming rules come from the conventions `C`.
pub struct MergePatch<C>(std::marker::PhantomData<C>);

impl<C: crate::conventions::Conventions> crate::steps::Derive for MergePatch<C> {
    type Input = parse::merge_patch::Input;
    type TopLevelMeta = parse_top_level_meta::merge_patch::TopLevelMeta;
    type Parsed<'i> = parse::merge_patch::Parsed<'i>;
    type MappingPaths = mapping::merge_patch::MappingPaths;
    type Model<'p> = analyze::merge_patch::Model;
    type Ir<'p> = lower::merge_patch::IntermediaryRepresentation<'p>;

    fn input(derive_input: syn::DeriveInput) -> Self::Input {
        let syn::DeriveInput { ident, data, .. } = derive_input;

        Self::Input::new(ident, data, "MergePatch")
    }

    fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>> {
        parse::merge_patch::parse::<C>(input)
    }

    fn map(
        _parsed: &Self::Parsed<'_>,
        top_level_meta: Self::TopLevelMeta,
    ) -> syn::Result<Self::MappingPaths> {
        mapping::merge_patch::generate_paths::<C>(
            top_level_meta.merge_patch_model,
            top_level_meta.runtime_crate,
        )
    }

    fn analyze<'p>(parsed: &'p Self::Parsed<'_>) -> syn::Result<Self::Model<'p>> {
        analyze::merge_patch::analyze(parsed)
    }

    fn lower<'p>(model: Self::Model<'p>, parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p> {
        lower::merge_patch::lower(model, parsed.ident)
    }

    fn code_gen(ir: Self::Ir<'_>, mapping_paths: Self::MappingPaths) -> proc_macro2::TokenStream {
        code_gen::merge_patch::code_gen(ir, mapping_paths)
    }

    fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path {
        &mapping_paths.runtime
    }
}
