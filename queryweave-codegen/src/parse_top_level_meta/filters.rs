/// The integrations a mirror's generated code opts into: the flags written
/// in `#[filters(...)]` (so an error about one can point at it), each naming
/// one of the conventions' integrations. A mirror without any is plain: it
/// names only the core runtime.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Integrations {
    /// The flags written, in the order of the conventions' integrations.
    flags: Vec<syn::Ident>,
}

impl Integrations {
    /// The integrations whose flags are `names`, as if written.
    #[cfg(test)]
    pub fn of(names: &[&str]) -> Self {
        Self {
            flags: names
                .iter()
                .map(|name| quote::format_ident!("{name}"))
                .collect(),
        }
    }

    /// The integrations whose flags are `flags`, read from an attribute
    /// (`#[validate_non_nullable(...)]`'s).
    #[cfg(feature = "merge-patch")]
    pub const fn from_flags(flags: Vec<syn::Ident>) -> Self {
        Self { flags }
    }

    /// The flags written.
    pub fn flags(&self) -> &[syn::Ident] {
        &self.flags
    }
}

/// The generated types a `Filters` struct renames, each with the name written
/// (`#[filters(query = UserSearch)]`), overriding its conventions' name.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Renames {
    /// `query`: a root model's query model.
    pub query: Option<syn::Ident>,
    /// `filter_options`: the filter options.
    pub filter_options: Option<syn::Ident>,
    /// `sort_field`: the sortable fields enum.
    pub sort_field: Option<syn::Ident>,
    /// `sort_details`: a sort field and its direction.
    pub sort_details: Option<syn::Ident>,
    /// `page`: a mirror's page alias.
    pub page: Option<syn::Ident>,
    /// `query_params`: a mirror's query parameters.
    pub query_params: Option<syn::Ident>,
}

impl Renames {
    /// The option of each generated type, with the name written for it.
    pub fn written(&self) -> impl Iterator<Item = (&'static str, &syn::Ident)> {
        [
            ("query", &self.query),
            ("filter_options", &self.filter_options),
            ("sort_field", &self.sort_field),
            ("sort_details", &self.sort_details),
            ("page", &self.page),
            ("query_params", &self.query_params),
        ]
        .into_iter()
        .filter_map(|(key, name)| name.as_ref().map(|name| (key, name)))
    }
}

/// The options of `#[filters(...)]` renaming a generated type.
const RENAME_OPTIONS: [&str; 6] = [
    "query",
    "filter_options",
    "sort_field",
    "sort_details",
    "page",
    "query_params",
];

/// The struct attributes of a `Filters` struct, whose integrations are the
/// conventions `C`'s.
pub struct TopLevelMeta<C> {
    /// `#[filters(mirrors = path)]`: the model (deriving `Filters`: a root
    /// model, or another mirror) the struct mirrors, whose generated types
    /// its own convert into. Without it, the struct is a root model.
    pub mirrors: Option<syn::Path>,
    /// `#[filters(serde, utoipa, ...)]`: the integrations of a mirror.
    pub integrations: Integrations,
    /// `#[filters(query = Name, ...)]`: the generated types renamed.
    pub renames: Renames,
    /// `#[queryweave(crate = path)]`: where the runtime crate the generated
    /// code names lives, overriding the conventions' default.
    pub runtime_crate: Option<syn::Path>,
    /// The conventions whose integrations the flags name.
    conventions: std::marker::PhantomData<fn() -> C>,
}

/// Read `#[filters(...)]`: `mirrors`, a flag per integration of `registry`
/// (whose order the flags keep), and the renamed generated types.
fn read_filters(
    attributes: &[syn::Attribute],
    registry: &[&'static dyn crate::integrations::Integration],
) -> syn::Result<(Option<syn::Path>, Integrations, Renames)> {
    crate::integrations::check_registry(registry)?;
    let options = std::iter::once(("mirrors", crate::attributes::OptionKind::Path))
        .chain(
            RENAME_OPTIONS
                .into_iter()
                .map(|key| (key, crate::attributes::OptionKind::Path)),
        )
        .chain(
            registry
                .iter()
                .map(|integration| (integration.name(), crate::attributes::OptionKind::Flag)),
        )
        .collect::<Vec<_>>();

    let mirrors = crate::attributes::path_option(
        attributes,
        "filters",
        "mirrors",
        &options,
        "Name the model (deriving Filters) this struct mirrors: #[filters(mirrors = path)]",
    )?;
    let mut flags = Vec::new();
    for integration in registry {
        flags.extend(crate::attributes::flag_option(
            attributes,
            "filters",
            integration.name(),
            &options,
        )?);
    }

    let rename = |key| {
        crate::attributes::ident_option(
            attributes,
            "filters",
            key,
            &options,
            "Name the generated type, e.g. #[filters(query = UserSearch)]",
        )
    };
    let renames = Renames {
        query: rename("query")?,
        filter_options: rename("filter_options")?,
        sort_field: rename("sort_field")?,
        sort_details: rename("sort_details")?,
        page: rename("page")?,
        query_params: rename("query_params")?,
    };

    Ok((mirrors, Integrations { flags }, renames))
}

impl<C: crate::conventions::Conventions> crate::steps::TopLevelMeta for TopLevelMeta<C> {
    fn from_attributes(attributes: &[syn::Attribute]) -> syn::Result<Self> {
        // One attribute, read option by option: its first error is enough
        // (each read checks every option).
        let filters = read_filters(attributes, &C::integrations());
        let runtime_crate = crate::attributes::crate_option(attributes);

        // Report every malformed attribute at once.
        let ((mirrors, integrations, renames), runtime_crate) =
            crate::errors::both(filters, runtime_crate)?;

        Ok(Self {
            mirrors,
            integrations,
            renames,
            runtime_crate,
            conventions: std::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {

    type Fields = (Option<syn::Path>, super::Integrations, Option<syn::Path>);

    /// The meta read from `item`'s attributes, under the default conventions,
    /// as its fields.
    fn parse(item: proc_macro2::TokenStream) -> syn::Result<Fields> {
        let attributes = syn::parse2::<syn::DeriveInput>(item).unwrap().attrs;
        let meta: super::TopLevelMeta<crate::conventions::DefaultConventions> =
            crate::steps::TopLevelMeta::from_attributes(&attributes)?;

        Ok((meta.mirrors, meta.integrations, meta.runtime_crate))
    }

    #[test]
    fn test_parsing_top_level_meta_reads_the_mirrored_model() {
        let top_level_meta = parse(quote::quote! {
            #[derive(Filters)]
            #[filters(mirrors = domain::models::user::UserModel)]
            struct UserDto {
                #[filter(sw)]
                #[sortable]
                username: String,
            }
        })
        .unwrap();

        pretty_assertions::assert_eq!(
            top_level_meta,
            (
                Some(syn::parse_str("domain::models::user::UserModel").unwrap()),
                super::Integrations::default(),
                None
            )
        );
    }

    #[test]
    fn test_parsing_top_level_meta_without_the_attribute_is_a_root_model() {
        let top_level_meta = parse(quote::quote! {
            #[derive(Filters)]
            struct UserModel {
                #[filter(sw)]
                username: String,
            }
        })
        .unwrap();

        pretty_assertions::assert_eq!(top_level_meta, (None, super::Integrations::default(), None));
    }

    // Each flag is read with the ident written, next to `mirrors` in any order.
    #[test]
    fn test_parsing_top_level_meta_reads_the_integrations() {
        let top_level_meta = parse(quote::quote! {
            #[filters(serde, mirrors = UserModel, validator)]
            struct UserDto;
        })
        .unwrap();

        pretty_assertions::assert_eq!(
            top_level_meta,
            (
                Some(syn::parse_quote!(UserModel)),
                super::Integrations::of(&["serde", "validator"]),
                None
            )
        );
    }

    // The flags are the conventions' integrations: a crate's own (e.g. one
    // replacing serde) is read like a built-in one.
    #[test]
    fn test_parsing_top_level_meta_reads_the_flags_of_the_given_integrations() {
        struct Wire;

        impl crate::integrations::Integration for Wire {
            fn name(&self) -> &'static str {
                "wire"
            }
        }

        let item: syn::DeriveInput = syn::parse_quote! {
            #[filters(mirrors = UserModel, wire)]
            struct UserDto;
        };

        let (_, integrations, _) = super::read_filters(&item.attrs, &[&Wire]).unwrap();

        pretty_assertions::assert_eq!(integrations, super::Integrations::of(&["wire"]));
        assert!(super::read_filters(&item.attrs, &crate::integrations::BUILT_IN).is_err());
    }

    // A generated type is renamed with a name, next to the other options.
    #[test]
    fn test_parsing_top_level_meta_reads_the_renames() {
        let item: syn::DeriveInput = syn::parse_quote! {
            #[filters(query = UserSearch, mirrors = UserModel, sort_field = UserSortKey)]
            struct UserDto;
        };

        let (_, _, renames) =
            super::read_filters(&item.attrs, &crate::integrations::BUILT_IN).unwrap();

        pretty_assertions::assert_eq!(
            renames,
            super::Renames {
                query: Some(quote::format_ident!("UserSearch")),
                sort_field: Some(quote::format_ident!("UserSortKey")),
                ..Default::default()
            }
        );
    }

    // A rename takes a single name: not a path, not a flag.
    #[test]
    fn test_parsing_top_level_meta_fails_for_a_malformed_rename() {
        for item in [
            quote::quote! { #[filters(query = search::UserSearch)] struct UserModel; },
            quote::quote! { #[filters(query)] struct UserModel; },
            quote::quote! { #[filters(query = A, query = B)] struct UserModel; },
        ] {
            assert!(parse(item).is_err());
        }
    }

    #[test]
    fn test_parsing_top_level_meta_fails_for_a_malformed_integration() {
        for item in [
            quote::quote! { #[filters(mirrors = UserModel, serde = true)] struct UserDto; },
            quote::quote! { #[filters(mirrors = UserModel, utoipa, utoipa)] struct UserDto; },
            quote::quote! { #[filters(mirrors = UserModel, serdes)] struct UserDto; },
        ] {
            assert!(parse(item).is_err());
        }
    }

    #[test]
    fn test_parsing_top_level_meta_fails_for_a_malformed_mirrors_option() {
        for item in [
            quote::quote! { #[filters()] struct UserDto; },
            quote::quote! { #[filters(mirrors)] struct UserDto; },
            quote::quote! { #[filters(mirrors = domain::models::user::)] struct UserDto; },
            quote::quote! { #[filters(mirrors = UserModel, mirrors = OtherModel)] struct UserDto; },
            quote::quote! { #[filters(mirror = UserModel)] struct UserDto; },
        ] {
            assert!(parse(item).is_err());
        }
    }
}
