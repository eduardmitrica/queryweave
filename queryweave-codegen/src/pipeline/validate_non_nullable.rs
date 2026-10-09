use crate::{analyze, code_gen, lower, mapping, parse, parse_top_level_meta};

/// `#[derive(ValidateNonNullable)]` on a merge patch DTO, linked to its domain
/// merge patch model.
///
/// Its naming rules come from the conventions `C`.
pub struct ValidateNonNullable<C>(std::marker::PhantomData<C>);

impl<C: crate::conventions::Conventions> crate::steps::Derive for ValidateNonNullable<C> {
    type Input = parse::validate_non_nullable::Input;
    type TopLevelMeta = parse_top_level_meta::validate_non_nullable::TopLevelMeta<C>;
    type Parsed<'i> = parse::validate_non_nullable::Parsed<'i>;
    type MappingPaths = mapping::validate_non_nullable::MappingPaths;
    type Model<'p> = analyze::validate_non_nullable::Model;
    type Ir<'p> = lower::validate_non_nullable::IntermediaryRepresentation<'p>;

    fn input(derive_input: syn::DeriveInput) -> Self::Input {
        let syn::DeriveInput { ident, data, .. } = derive_input;

        Self::Input::new(ident, data, "ValidateNonNullable")
    }

    fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>> {
        parse::validate_non_nullable::parse::<C>(input)
    }

    fn map(
        parsed: &Self::Parsed<'_>,
        top_level_meta: Self::TopLevelMeta,
    ) -> syn::Result<Self::MappingPaths> {
        mapping::validate_non_nullable::generate_paths::<C>(
            parsed.ident,
            top_level_meta.mirrors,
            &top_level_meta.integrations,
            top_level_meta.runtime_crate,
        )
    }

    fn analyze<'p>(parsed: &'p Self::Parsed<'_>) -> syn::Result<Self::Model<'p>> {
        analyze::validate_non_nullable::analyze(parsed)
    }

    fn lower<'p>(model: Self::Model<'p>, parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p> {
        lower::validate_non_nullable::lower(model, parsed.ident)
    }

    fn code_gen(ir: Self::Ir<'_>, mapping_paths: Self::MappingPaths) -> proc_macro2::TokenStream {
        code_gen::validate_non_nullable::code_gen(ir, mapping_paths)
    }

    fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path {
        &mapping_paths.runtime
    }
}
