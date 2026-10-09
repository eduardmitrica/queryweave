use crate::{code_gen, mapping, parse, parse_top_level_meta};

/// `#[derive(Columns)]` on a `sqlx` entity. Nothing to analyze or lower: the
/// columns are read while parsing.
///
/// The paths of the runtime crates it names come from the conventions `C`.
pub struct Columns<C>(std::marker::PhantomData<C>);

impl<C: crate::conventions::Conventions> crate::steps::Derive for Columns<C> {
    type Input = parse::columns::Input;
    type TopLevelMeta = parse_top_level_meta::columns::TopLevelMeta;
    type Parsed<'i> = parse::columns::Parsed<'i>;
    type MappingPaths = mapping::columns::MappingPaths;
    type Model<'p> = ();
    type Ir<'p> = &'p parse::columns::Parsed<'p>;

    fn input(derive_input: syn::DeriveInput) -> Self::Input {
        let syn::DeriveInput {
            ident,
            generics,
            data,
            attrs,
            ..
        } = derive_input;

        // The entity's own attributes stay with the input: `#[sqlx(...)]`
        // among them is part of what is parsed.
        Self::Input::new(ident, generics, data, attrs)
    }

    fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>> {
        parse::columns::parse(input)
    }

    fn map(
        _parsed: &Self::Parsed<'_>,
        top_level_meta: Self::TopLevelMeta,
    ) -> syn::Result<Self::MappingPaths> {
        Ok(mapping::columns::generate_paths::<C>(top_level_meta))
    }

    fn analyze(_parsed: &Self::Parsed<'_>) -> syn::Result<()> {
        Ok(())
    }

    fn lower<'p>((): (), parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p> {
        parsed
    }

    fn code_gen(ir: Self::Ir<'_>, mapping_paths: Self::MappingPaths) -> proc_macro2::TokenStream {
        code_gen::columns::code_gen(ir, mapping_paths)
    }

    fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path {
        &mapping_paths.runtime
    }
}
