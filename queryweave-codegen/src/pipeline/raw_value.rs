use crate::{code_gen, mapping, parse, parse_top_level_meta};

/// `#[derive(RawValue)]` on a value object. Nothing to analyze or lower: the
/// code is generated from the parsed newtype.
///
/// The paths of the runtime crates it names come from the conventions `C`.
pub struct RawValue<C>(std::marker::PhantomData<C>);

impl<C: crate::conventions::Conventions> crate::steps::Derive for RawValue<C> {
    type Input = parse::raw_value::Input;
    type TopLevelMeta = parse_top_level_meta::raw_value::TopLevelMeta;
    type Parsed<'i> = parse::raw_value::Parsed<'i>;
    type MappingPaths = mapping::raw_value::MappingPaths;
    type Model<'p> = ();
    type Ir<'p> = &'p parse::raw_value::Parsed<'p>;

    fn input(derive_input: syn::DeriveInput) -> Self::Input {
        let syn::DeriveInput {
            ident,
            generics,
            data,
            ..
        } = derive_input;

        Self::Input::new(ident, generics, data)
    }

    fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>> {
        parse::raw_value::parse(input)
    }

    fn map(
        _parsed: &Self::Parsed<'_>,
        top_level_meta: Self::TopLevelMeta,
    ) -> syn::Result<Self::MappingPaths> {
        Ok(mapping::raw_value::generate_paths::<C>(
            top_level_meta.runtime_crate,
        ))
    }

    fn analyze(_parsed: &Self::Parsed<'_>) -> syn::Result<()> {
        Ok(())
    }

    fn lower<'p>((): (), parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p> {
        parsed
    }

    fn code_gen(ir: Self::Ir<'_>, mapping_paths: Self::MappingPaths) -> proc_macro2::TokenStream {
        code_gen::raw_value::code_gen(ir, &mapping_paths)
    }

    fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path {
        &mapping_paths.runtime
    }
}
