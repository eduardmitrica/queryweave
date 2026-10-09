//! The pipeline every derive is built as.
//!
//! A derive implements [`Derive`], whose associated types are its stages, and
//! its `#[proc_macro_derive]` entry point is a single [`expand`] call. The
//! stages run in a fixed order:
//!
//! 1. the top-level meta ([`TopLevelMeta`]) is read from the struct
//!    attributes, and the input is parsed ([`Derive::parse`]);
//! 2. the top-level meta is mapped onto the paths the generated code links to
//!    ([`Derive::map`]), and the parsed input is analyzed into a model
//!    ([`Derive::analyze`]);
//! 3. the model is lowered into an intermediary representation
//!    ([`Derive::lower`]);
//! 4. the intermediary representation and the mapping paths are turned into
//!    code ([`Derive::code_gen`]).
//!
//! The stages of each numbered step do not depend on each other, so their
//! errors are reported together. A derive skips a stage with an identity type:
//! `()` for no top-level meta or mapping paths or model, and the parsed input
//! (by reference) as its model or intermediary representation. (Every
//! `queryweave` derive reads `#[queryweave(crate = path)]` and maps its runtime
//! crate, so they only ever skip the model.)

/// What a derive reads from the attributes on the struct it is placed on
/// (e.g. `#[merge_patch(patch = path)]`).
///
/// Each derive with struct attributes defines its own `TopLevelMeta` struct in
/// its `parse_top_level_meta` module, one field per attribute option, each
/// read with the option readers of [`crate::attributes`]
/// ([`crate::attributes::path_option`], ...). `()` is the meta of a derive
/// without struct attributes.
pub trait TopLevelMeta: Sized {
    /// Read the meta from the struct attributes.
    ///
    /// # Errors
    ///
    /// Returns the errors of missing, repeated or malformed struct attributes.
    fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self>;
}

impl TopLevelMeta for () {
    // equivalent mutant: `Ok(Default::default())` is `Ok(())`, the only value
    #[mutants::skip]
    fn from_attributes(_attributes: &[syn::Attribute]) -> syn::Result<Self> {
        Ok(())
    }
}

/// A derive, as the types of its stages and the functions between them.
///
/// See the [module documentation](self) for the order the stages run in.
/// Each function delegates to the step module of its stage.
pub trait Derive {
    /// The derive input, as the parse step needs it.
    type Input;
    /// The struct attributes the derive reads.
    type TopLevelMeta: TopLevelMeta;
    /// The parsed input, borrowing from [`Derive::Input`].
    type Parsed<'i>;
    /// The paths of the types the generated code links to.
    type MappingPaths;
    /// The analyzed input, possibly borrowing from [`Derive::Parsed`].
    type Model<'p>;
    /// The intermediary representation code is generated from.
    type Ir<'p>;

    /// Build the input from the derive's syntax tree (its attributes having
    /// been read into [`Derive::TopLevelMeta`]).
    fn input(derive_input: syn::DeriveInput) -> Self::Input;

    /// Parse the input.
    ///
    /// # Errors
    ///
    /// Returns the errors of an input the derive cannot be placed on.
    fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>>;

    /// Map the top-level meta onto the paths the generated code links to.
    ///
    /// # Errors
    ///
    /// Returns the errors of a path that does not name what it should.
    fn map(
        parsed: &Self::Parsed<'_>,
        top_level_meta: Self::TopLevelMeta,
    ) -> syn::Result<Self::MappingPaths>;

    /// Analyze the parsed input into a model.
    ///
    /// # Errors
    ///
    /// Returns the errors of the analysis, e.g. malformed field attributes.
    fn analyze<'p>(parsed: &'p Self::Parsed<'_>) -> syn::Result<Self::Model<'p>>;

    /// Lower the model into the intermediary representation.
    fn lower<'p>(model: Self::Model<'p>, parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p>;

    /// Generate the code.
    fn code_gen(ir: Self::Ir<'_>, mapping_paths: Self::MappingPaths) -> proc_macro2::TokenStream;

    /// The runtime crate the generated code names, as mapped.
    fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path;
}

/// Run the derive `D` on the tokens of the item it is placed on: its
/// `#[proc_macro_derive]` entry point is
/// `expand::<D>(input.into()).into()`.
///
/// Errors come back as `compile_error!` invocations, all of those of the
/// stages that ran.
#[must_use]
pub fn expand<D: Derive>(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    syn::parse2::<syn::DeriveInput>(input)
        .and_then(|derive_input| {
            let top_level_meta = D::TopLevelMeta::from_attributes(&derive_input.attrs);
            let input = D::input(derive_input);

            run::<D>(&input, top_level_meta)
        })
        .unwrap_or_else(syn::Error::into_compile_error)
}

/// The stages after reading the top-level meta.
fn run<D: Derive>(
    input: &D::Input,
    top_level_meta: syn::Result<D::TopLevelMeta>,
) -> syn::Result<proc_macro2::TokenStream> {
    let (parsed, top_level_meta) = crate::errors::both(D::parse(input), top_level_meta)?;
    let (mapping_paths, model) =
        crate::errors::both(D::map(&parsed, top_level_meta), D::analyze(&parsed))?;
    let ir = D::lower(model, &parsed);
    let version_check = version_check(D::runtime(&mapping_paths));

    let mut code = D::code_gen(ir, mapping_paths);
    code.extend(version_check);
    Ok(code)
}

/// A check, run when the generated code is compiled, that the runtime crate
/// it names is compatible with this version of the code generation: the
/// generated code names runtime items of its own version, so a mismatch
/// would otherwise surface as unrelated missing items.
fn version_check(runtime: &syn::Path) -> proc_macro2::TokenStream {
    let major = env!("CARGO_PKG_VERSION_MAJOR")
        .parse::<u64>()
        .map_or_else(|_| quote::quote!(0), |major| quote::quote!(#major));
    let minor = env!("CARGO_PKG_VERSION_MINOR")
        .parse::<u64>()
        .map_or_else(|_| quote::quote!(0), |minor| quote::quote!(#minor));

    quote::quote! {
        const _: () = #runtime::__private::require_version(#major, #minor);
    }
}

#[cfg(test)]
mod tests {
    use super::{Derive, TopLevelMeta, expand};

    /// The name of a struct attribute, if the struct has one.
    struct Meta(Option<syn::Ident>);

    impl TopLevelMeta for Meta {
        fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self> {
            match attributes.first() {
                Some(attribute) if attribute.path().is_ident("bad_meta") => {
                    Err(syn::Error::new_spanned(attribute, "bad meta"))
                },
                attribute => {
                    Ok(Self(attribute.and_then(|attribute| {
                        attribute.path().get_ident().cloned()
                    })))
                },
            }
        }
    }

    /// A derive recording which stages ran, failing a stage when the struct
    /// is named after it.
    struct Recorder;

    impl Derive for Recorder {
        type Input = syn::DeriveInput;
        type TopLevelMeta = Meta;
        type Parsed<'i> = &'i syn::Ident;
        type MappingPaths = (Option<syn::Ident>, syn::Path);
        type Model<'p> = String;
        type Ir<'p> = String;

        fn input(derive_input: syn::DeriveInput) -> Self::Input {
            derive_input
        }

        fn parse(input: &Self::Input) -> syn::Result<Self::Parsed<'_>> {
            if input.ident == "BadParse" {
                return Err(syn::Error::new_spanned(&input.ident, "bad parse"));
            }

            Ok(&input.ident)
        }

        fn map(
            parsed: &Self::Parsed<'_>,
            top_level_meta: Self::TopLevelMeta,
        ) -> syn::Result<Self::MappingPaths> {
            if *parsed == "BadMapAndAnalyze" {
                return Err(syn::Error::new_spanned(parsed, "bad map"));
            }

            Ok((top_level_meta.0, syn::parse_quote!(::runtime)))
        }

        fn analyze<'p>(parsed: &'p Self::Parsed<'_>) -> syn::Result<Self::Model<'p>> {
            if *parsed == "BadMapAndAnalyze" {
                return Err(syn::Error::new_spanned(parsed, "bad analyze"));
            }

            Ok(parsed.to_string())
        }

        fn lower<'p>(model: Self::Model<'p>, _parsed: &'p Self::Parsed<'_>) -> Self::Ir<'p> {
            format!("{model}_lowered")
        }

        fn code_gen(
            ir: Self::Ir<'_>,
            mapping_paths: Self::MappingPaths,
        ) -> proc_macro2::TokenStream {
            let ir = quote::format_ident!("{ir}");
            let linked = mapping_paths.0;
            quote::quote!(struct #ir; #linked)
        }

        fn runtime(mapping_paths: &Self::MappingPaths) -> &syn::Path {
            &mapping_paths.1
        }
    }

    fn expanded(item: proc_macro2::TokenStream) -> String {
        expand::<Recorder>(item).to_string()
    }

    #[test]
    fn every_stage_runs_in_order() {
        assert_eq!(
            expanded(quote::quote!(
                #[linked]
                struct Order;
            )),
            format!(
                "struct Order_lowered ; linked const _ : () = :: runtime :: __private :: require_version ({}u64 , {}u64) ;",
                env!("CARGO_PKG_VERSION_MAJOR"),
                env!("CARGO_PKG_VERSION_MINOR"),
            )
        );
    }

    // The top-level meta and the parse step fail independently, and both
    // failures are reported.
    #[test]
    fn meta_and_parse_errors_are_reported_together() {
        let errors = expanded(quote::quote!(
            #[bad_meta]
            struct BadParse;
        ));

        assert!(errors.contains("bad parse"));
        assert!(errors.contains("bad meta"));
    }

    #[test]
    fn map_and_analyze_errors_are_reported_together() {
        let errors = expanded(quote::quote!(
            struct BadMapAndAnalyze;
        ));

        assert!(errors.contains("bad map"));
        assert!(errors.contains("bad analyze"));
    }

    #[test]
    fn tokens_that_are_not_an_item_are_rejected() {
        assert!(expanded(quote::quote!(1 + 1)).contains("compile_error"));
    }
}
