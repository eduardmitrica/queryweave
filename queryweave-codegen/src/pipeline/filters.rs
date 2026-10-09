use crate::{analyze, code_gen, lower, mapping, parse, parse_top_level_meta};

/// `#[derive(Filters)]` on a root model, or on a struct mirroring one
/// (`#[filters(mirrors = path)]`).
///
/// Its naming rules come from the conventions `C`.
pub struct Filters<C>(std::marker::PhantomData<C>);

impl<C: crate::conventions::Conventions> crate::steps::Derive for Filters<C> {
    type Input = parse::filters::Input;
    type TopLevelMeta = parse_top_level_meta::filters::TopLevelMeta<C>;
    type Parsed<'i> = parse::filters::Parsed<'i>;
    type MappingPaths = mapping::filters::MappingPaths;
    type Model<'p> = analyze::filters::Model;
    type Ir<'p> = lower::filters::IntermediaryRepresentation;

    fn input(derive_input: syn::DeriveInput) -> Self::Input {
        let syn::DeriveInput { ident, data, .. } = derive_input;

        Self::Input::new(ident, data, "Filters")
    }

    fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>> {
        parse::filters::parse::<C>(input)
    }

    fn map(
        parsed: &Self::Parsed<'_>,
        top_level_meta: Self::TopLevelMeta,
    ) -> syn::Result<Self::MappingPaths> {
        mapping::filters::generate_paths::<C>(
            parsed,
            top_level_meta.mirrors,
            &top_level_meta.integrations,
            &top_level_meta.renames,
            top_level_meta.runtime_crate,
        )
    }

    fn analyze<'p>(parsed: &'p Self::Parsed<'_>) -> syn::Result<Self::Model<'p>> {
        analyze::filters::analyze::<C>(parsed)
    }

    fn lower<'p>(model: Self::Model<'p>, parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p> {
        lower::filters::lower(model, parsed.idents.clone())
    }

    fn code_gen(ir: Self::Ir<'_>, mapping_paths: Self::MappingPaths) -> proc_macro2::TokenStream {
        code_gen::filters::code_gen(ir, &mapping_paths)
    }

    fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path {
        &mapping_paths.runtime
    }
}
