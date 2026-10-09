use crate::{analyze, code_gen, lower, mapping, parse, parse_top_level_meta};

/// `#[derive(MergePatchNullableFields)]` on a merge patch model.
///
/// Its naming rules come from the conventions `C`.
pub struct MergePatchNullableFields<C>(std::marker::PhantomData<C>);

impl<C: crate::conventions::Conventions> crate::steps::Derive for MergePatchNullableFields<C> {
    type Input = parse::merge_patch_nullable_fields::Input;
    type TopLevelMeta = parse_top_level_meta::merge_patch_nullable_fields::TopLevelMeta;
    type Parsed<'i> = parse::merge_patch_nullable_fields::Parsed<'i>;
    type MappingPaths = mapping::merge_patch_nullable_fields::MappingPaths;
    type Model<'p> = analyze::merge_patch_nullable_fields::Model<'p>;
    type Ir<'p> = lower::merge_patch_nullable_fields::IntermediaryRepresentation<'p>;

    fn input(derive_input: syn::DeriveInput) -> Self::Input {
        let syn::DeriveInput { ident, data, .. } = derive_input;

        Self::Input::new(ident, data, "MergePatchNullableFields")
    }

    fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>> {
        parse::merge_patch_nullable_fields::parse::<C>(input)
    }

    fn map(
        _parsed: &Self::Parsed<'_>,
        top_level_meta: Self::TopLevelMeta,
    ) -> syn::Result<Self::MappingPaths> {
        Ok(mapping::merge_patch_nullable_fields::generate_paths::<C>(
            top_level_meta.runtime_crate,
        ))
    }

    fn analyze<'p>(parsed: &'p Self::Parsed<'_>) -> syn::Result<Self::Model<'p>> {
        analyze::merge_patch_nullable_fields::analyze(parsed)
    }

    fn lower<'p>(model: Self::Model<'p>, _parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p> {
        lower::merge_patch_nullable_fields::lower(model)
    }

    fn code_gen(ir: Self::Ir<'_>, mapping_paths: Self::MappingPaths) -> proc_macro2::TokenStream {
        code_gen::merge_patch_nullable_fields::code_gen(ir, &mapping_paths)
    }

    fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path {
        &mapping_paths.runtime
    }
}
