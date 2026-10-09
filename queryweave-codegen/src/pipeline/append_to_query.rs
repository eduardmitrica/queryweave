use crate::{code_gen, mapping, parse, parse_top_level_meta};

/// `#[derive(AppendToQuery)]` on a wrapper of a domain query model. Nothing to
/// analyze or lower: the code is generated from the parsed wrapper.
///
/// Its naming rules come from the conventions `C`.
pub struct AppendToQuery<C>(std::marker::PhantomData<C>);

impl<C: crate::conventions::Conventions> crate::steps::Derive for AppendToQuery<C> {
    type Input = parse::append_to_query_shared::Input;
    type TopLevelMeta = parse_top_level_meta::append_to_query::TopLevelMeta;
    type Parsed<'i> = parse::append_to_query_shared::Parsed<'i>;
    type MappingPaths = mapping::append_to_query::MappingPaths;
    type Model<'p> = ();
    type Ir<'p> = &'p parse::append_to_query_shared::Parsed<'p>;

    fn input(derive_input: syn::DeriveInput) -> Self::Input {
        let syn::DeriveInput { ident, data, .. } = derive_input;

        Self::Input::new(ident, data)
    }

    fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>> {
        parse::append_to_query_shared::parse(input)
    }

    fn map(
        _parsed: &Self::Parsed<'_>,
        top_level_meta: Self::TopLevelMeta,
    ) -> syn::Result<Self::MappingPaths> {
        mapping::append_to_query::generate_paths::<C>(top_level_meta)
    }

    fn analyze(_parsed: &Self::Parsed<'_>) -> syn::Result<()> {
        Ok(())
    }

    fn lower<'p>((): (), parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p> {
        parsed
    }

    fn code_gen(ir: Self::Ir<'_>, mapping_paths: Self::MappingPaths) -> proc_macro2::TokenStream {
        code_gen::append_to_query::code_gen(ir, mapping_paths)
    }

    fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path {
        &mapping_paths.runtime
    }
}
