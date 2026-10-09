/// What a `Filters` struct is, which decides what is generated for it.
#[derive(Debug, PartialEq, Eq)]
pub enum Role {
    /// A root model: it owns its generated types and publishes them through
    /// `queryweave::Filterable`.
    Root,
    /// A mirror of `model` (deriving `Filters`: a root model, or another
    /// mirror), named by
    /// `#[filters(mirrors = path)]`: its generated types convert into the
    /// model's, which it names through `queryweave::Filterable`, never by
    /// rebuilding their names.
    Mirror {
        /// The mirrored model.
        model: syn::Path,
        /// The integrations its generated code opts into, in the order of
        /// the conventions' integrations.
        integrations: Vec<crate::integrations::Active>,
    },
}

/// What the code generated for a `Filters` struct links to.
#[derive(Debug, PartialEq, Eq)]
pub struct MappingPaths {
    /// The struct's role.
    pub role: Role,
    /// The path of the runtime crate the generated code names: the one given
    /// by `#[queryweave(crate = path)]`, or the conventions' default:
    /// `C::api_crate()` for a mirror with an integration naming the API
    /// runtime (e.g. `serde`), `C::core_crate()` otherwise.
    pub runtime: syn::Path,
    /// The names of the generated types: the conventions', with the struct's
    /// renames (`#[filters(query = Name, ...)]`).
    pub names: crate::conventions::FiltersNames,
}

/// The generated types of each role, by their `#[filters(...)]` option.
const ROOT_TYPES: [&str; 4] = ["query", "filter_options", "sort_field", "sort_details"];
const MIRROR_TYPES: [&str; 5] = [
    "query_params",
    "filter_options",
    "sort_field",
    "sort_details",
    "page",
];

/// The name `key`'s generated type has in `names`.
fn name_of<'a>(names: &'a crate::conventions::FiltersNames, key: &str) -> &'a syn::Ident {
    match key {
        "query" => &names.query,
        "filter_options" => &names.filters,
        "sort_field" => &names.sort_field,
        "sort_details" => &names.sort_details,
        "page" => &names.page,
        _ => &names.query_params,
    }
}

/// The names of the generated types of a struct with the role's `types`:
/// the conventions' `names`, with `renames`.
///
/// # Errors
///
/// Returns an error at each rename of a type the role does not generate, and
/// at each generated type whose name another one (or the struct) has.
fn apply_renames(
    mut names: crate::conventions::FiltersNames,
    renames: &crate::parse_top_level_meta::filters::Renames,
    types: &[&str],
) -> syn::Result<crate::conventions::FiltersNames> {
    let mut errors = crate::errors::Errors::default();
    for (key, name) in renames.written() {
        if !types.contains(&key) {
            let (role, generates) = if types.contains(&"query") {
                ("a root model", "A root model generates")
            } else {
                ("a mirror", "A mirror generates")
            };
            errors.push(syn::Error::new(
                name.span(),
                crate::errors::message(
                    &format!("`{key}` names a type {role} does not generate"),
                    &format!("{generates}: {}", types.join(", ")),
                ),
            ));
            continue;
        }
        let slot = match key {
            "query" => &mut names.query,
            "filter_options" => &mut names.filters,
            "sort_field" => &mut names.sort_field,
            "sort_details" => &mut names.sort_details,
            "page" => &mut names.page,
            _ => &mut names.query_params,
        };
        *slot = name.clone();
    }
    errors.finish(())?;

    // Every generated type needs a name of its own, other than the struct's.
    let mut errors = crate::errors::Errors::default();
    let mut taken = vec![(String::from("the struct itself"), &names.input)];
    for key in types {
        let name = name_of(&names, key);
        if let Some((other, _)) = taken.iter().find(|(_, taken)| *taken == name) {
            errors.push(syn::Error::new(
                name.span(),
                crate::errors::message(
                    &format!("The generated `{key}` type and {other} are both named `{name}`"),
                    &format!("Rename one of them, e.g. #[filters({key} = OtherName)]"),
                ),
            ));
        }
        taken.push((format!("the generated `{key}` type"), name));
    }

    errors.finish(names)
}

/// The struct's role, from `#[filters(mirrors = path)]`, and the runtime crate
/// its generated code names.
///
/// A root model maps onto no other type, so it cannot rename a field onto one
/// with `#[equivalent_field(name)]`, and has no integrations. A mirror's
/// integrations (the conventions' `C::integrations()` its flags name) must
/// each have the ones it requires, and agree on the sort direction and page
/// types they provide.
///
/// # Errors
///
/// Returns an error at each `#[equivalent_field(...)]` and integration flag
/// of a root model, and at each flag of a mirror lacking a required
/// integration or disagreeing with an earlier one.
pub fn generate_paths<C: crate::conventions::Conventions>(
    parsed: &crate::parse::filters::Parsed<'_>,
    mirrors: Option<syn::Path>,
    integrations: &crate::parse_top_level_meta::filters::Integrations,
    renames: &crate::parse_top_level_meta::filters::Renames,
    runtime_crate: Option<syn::Path>,
) -> syn::Result<MappingPaths> {
    let Some(model) = mirrors else {
        let (names, ()) = crate::errors::both(
            apply_renames(parsed.idents.clone(), renames, &ROOT_TYPES),
            check_root(parsed, integrations),
        )?;
        return Ok(MappingPaths {
            role: Role::Root,
            runtime: runtime_crate.unwrap_or_else(C::core_crate),
            names,
        });
    };

    crate::attributes::reject_self_link(&model, &parsed.idents.input, "filters", "mirrors")?;
    let names = apply_renames(parsed.idents.clone(), renames, &MIRROR_TYPES)?;
    let integrations = resolve(&C::integrations(), integrations);
    let runtime = runtime_crate.unwrap_or_else(|| {
        if integrations
            .iter()
            .any(|active| active.integration.names_api_runtime())
        {
            C::api_crate()
        } else {
            C::core_crate()
        }
    });
    check_integrations(&integrations, &names, &runtime)?;

    Ok(MappingPaths {
        role: Role::Mirror {
            model,
            integrations,
        },
        runtime,
        names,
    })
}

/// The integrations of `registry` the flags name, in `registry`'s order (the
/// flags were read against it, so each names one).
pub fn resolve(
    registry: &[&'static dyn crate::integrations::Integration],
    integrations: &crate::parse_top_level_meta::filters::Integrations,
) -> Vec<crate::integrations::Active> {
    registry
        .iter()
        .filter_map(|integration| {
            integrations
                .flags()
                .iter()
                .find(|flag| *flag == integration.name())
                .map(|flag| crate::integrations::Active {
                    integration: *integration,
                    flag: flag.clone(),
                })
        })
        .collect()
}

/// Each integration has the ones it requires, and they agree on the sort
/// direction and page types they provide.
fn check_integrations(
    integrations: &[crate::integrations::Active],
    names: &crate::conventions::FiltersNames,
    runtime: &syn::Path,
) -> syn::Result<()> {
    let mut errors = crate::errors::Errors::default();
    for crate::integrations::Active { integration, flag } in integrations {
        for (required, reason) in integration.requires() {
            if !integrations
                .iter()
                .any(|other| other.integration.name() == *required)
            {
                errors.push(syn::Error::new(
                    flag.span(),
                    crate::errors::message(
                        &format!("`{flag}` needs `{required}`: {reason}"),
                        &format!(
                            "Add `{required}` to the options: #[filters(mirrors = path, {required}, {flag})]"
                        ),
                    ),
                ));
            }
        }
    }
    let provided = integrations
        .iter()
        .map(|active| {
            let context = active.context(names, runtime);
            (
                &active.flag,
                active.integration.sort_direction(&context),
                active.integration.page(&context),
            )
        })
        .collect::<Vec<_>>();
    check_agreement(
        provided
            .iter()
            .map(|(flag, sort_direction, _)| (*flag, sort_direction.as_ref())),
        &mut errors,
        "sort direction",
    );
    check_agreement(
        provided
            .iter()
            .map(|(flag, _, page)| (*flag, page.as_ref())),
        &mut errors,
        "page type",
    );

    errors.finish(())
}

/// Report, at its flag, each integration providing a `what` other than the
/// first one's, given each flag with what its integration provides.
fn check_agreement<'a>(
    provided: impl Iterator<Item = (&'a syn::Ident, Option<&'a proc_macro2::TokenStream>)>,
    errors: &mut crate::errors::Errors,
    what: &str,
) {
    let mut first: Option<(&syn::Ident, String)> = None;
    for (flag, tokens) in provided {
        let Some(tokens) = tokens.map(ToString::to_string) else {
            continue;
        };
        match &first {
            None => first = Some((flag, tokens)),
            Some((first_flag, first_tokens)) if *first_tokens != tokens => {
                errors.push(syn::Error::new(
                    flag.span(),
                    crate::errors::message(
                        &format!(
                            "`{first_flag}` and `{flag}` need different {what}s: `{first_tokens}` and `{tokens}`"
                        ),
                        "Opt into integrations agreeing on it, e.g. by removing one of them",
                    ),
                ));
            },
            Some(_) => {},
        }
    }
}

/// A root model has no integrations and renames no field.
fn check_root(
    parsed: &crate::parse::filters::Parsed<'_>,
    integrations: &crate::parse_top_level_meta::filters::Integrations,
) -> syn::Result<()> {
    let mut errors = crate::errors::Errors::default();
    // A root model's generated types hand data to the persistence layer;
    // the integrations are for the types mirroring it (e.g. an API DTO).
    for flag in integrations.flags() {
        errors.push(syn::Error::new(
            flag.span(),
            crate::errors::message(
                &format!("`{flag}` only applies to a mirror (a struct with #[filters(mirrors = path)])"),
                "Link the struct to the model it mirrors with #[filters(mirrors = path)], or remove the option",
            ),
        ));
    }
    for attribute in parsed
        .named_fields
        .named
        .iter()
        .flat_map(|field| &field.attrs)
        .filter(|attribute| attribute.path().is_ident("equivalent_field"))
    {
        errors.push(syn::Error::new_spanned(
            attribute,
            crate::errors::message(
                "#[equivalent_field(...)] only applies to a mirror (a struct with #[filters(mirrors = path)])",
                "Link the struct to the model it mirrors with #[filters(mirrors = path)], or remove the attribute",
            ),
        ));
    }

    errors.finish(())
}

#[cfg(test)]
mod tests {

    fn parsed(named_fields: &syn::FieldsNamed) -> crate::parse::filters::Parsed<'_> {
        crate::parse::filters::Parsed {
            idents: <crate::conventions::DefaultConventions as crate::conventions::Conventions>::filters_names(
                &quote::format_ident!("UserDto"),
            )
            .unwrap(),
            named_fields,
        }
    }

    use crate::conventions::DefaultConventions;
    use crate::parse_top_level_meta::filters::Integrations;

    fn integrations(serde: bool, utoipa: bool, validator: bool) -> Integrations {
        let names = [
            ("serde", serde),
            ("utoipa", utoipa),
            ("validator", validator),
        ]
        .into_iter()
        .filter_map(|(name, written)| written.then_some(name))
        .collect::<Vec<_>>();
        Integrations::of(&names)
    }

    // A mirror (de)serialized or documented names the API runtime crate, the
    // core one otherwise.
    #[rstest::rstest]
    #[case::plain(integrations(false, false, false), syn::parse_quote!(::queryweave))]
    #[case::serde_and_validator(integrations(true, false, true), syn::parse_quote!(::queryweave_api))]
    #[case::serde(integrations(true, false, false), syn::parse_quote!(::queryweave_api))]
    #[case::utoipa(integrations(false, true, false), syn::parse_quote!(::queryweave_api))]
    #[case::all(integrations(true, true, true), syn::parse_quote!(::queryweave_api))]
    fn test_generating_paths_keeps_the_mirrored_model(
        #[case] integrations: Integrations,
        #[case] runtime: syn::Path,
    ) {
        let named_fields = syn::parse_quote!({ #[filter(sw)] username: String });
        let path = syn::parse_str::<syn::Path>("domain::models::user::UserModel").unwrap();

        pretty_assertions::assert_eq!(
            super::generate_paths::<DefaultConventions>(
                &parsed(&named_fields),
                Some(path.clone()),
                &integrations,
                &crate::parse_top_level_meta::filters::Renames::default(),
                None
            )
            .unwrap(),
            super::MappingPaths {
                role: super::Role::Mirror {
                    model: path,
                    integrations: super::resolve(&crate::integrations::BUILT_IN, &integrations),
                },
                runtime,
                names: parsed(&named_fields).idents,
            }
        );
    }

    // validator serializes the rejected value: it needs serde.
    #[rstest::rstest]
    #[case::alone(integrations(false, false, true))]
    #[case::with_utoipa(integrations(false, true, true))]
    fn test_generating_paths_rejects_validator_without_serde(#[case] integrations: Integrations) {
        let named_fields = syn::parse_quote!({ #[filter(sw)] username: String });

        assert!(
            super::generate_paths::<DefaultConventions>(
                &parsed(&named_fields),
                Some(syn::parse_quote!(UserModel)),
                &integrations,
                &crate::parse_top_level_meta::filters::Renames::default(),
                None,
            )
            .is_err()
        );
    }

    // The integrations are a mirror's: each one written on a root model is
    // reported.
    #[test]
    fn test_generating_paths_rejects_integrations_on_a_root() {
        let named_fields = syn::parse_quote!({ #[filter(sw)] username: String });

        let error = super::generate_paths::<DefaultConventions>(
            &parsed(&named_fields),
            None,
            &integrations(true, false, true),
            &crate::parse_top_level_meta::filters::Renames::default(),
            None,
        )
        .unwrap_err();

        assert_eq!(error.into_iter().count(), 2);
    }

    /// An integration named `name` providing `direction` as its sort direction.
    struct Direction(&'static str, &'static str);

    impl crate::integrations::Integration for Direction {
        fn name(&self) -> &'static str {
            self.0
        }

        fn sort_direction(
            &self,
            _context: &crate::integrations::Context<'_>,
        ) -> Option<proc_macro2::TokenStream> {
            self.1.parse().ok()
        }
    }

    static ONE: Direction = Direction("one", "crate::One");
    static SAME: Direction = Direction("same", "crate::One");
    static OTHER: Direction = Direction("other", "crate::Other");

    /// `check_integrations` on the integrations of `registry` named `names`.
    fn check(
        registry: &[&'static dyn crate::integrations::Integration],
        names: &[&str],
    ) -> syn::Result<()> {
        let idents = <DefaultConventions as crate::conventions::Conventions>::filters_names(
            &quote::format_ident!("UserDto"),
        )?;
        super::check_integrations(
            &super::resolve(registry, &Integrations::of(names)),
            &idents,
            &syn::parse_quote!(::queryweave),
        )
    }

    // The integrations providing a sort direction must agree on it; the ones
    // providing none follow.
    #[test]
    fn test_integrations_must_agree_on_the_sort_direction() {
        let registry: [&'static dyn crate::integrations::Integration; 4] =
            [&ONE, &SAME, &OTHER, crate::integrations::BUILT_IN[2]];

        assert!(check(&registry, &["one", "same"]).is_ok());
        assert!(check(&registry, &["one", "other"]).is_err());
        assert!(check(&registry, &["other"]).is_ok());
        // The built-in ones agree on `SortDirectionDto`.
        assert!(
            check(
                &crate::integrations::BUILT_IN,
                &["serde", "utoipa", "validator"]
            )
            .is_ok()
        );
    }

    /// `generate_paths` on `UserDto`, a root or a mirror, with `renames`.
    fn with_renames(
        mirrors: Option<&str>,
        renames: &crate::parse_top_level_meta::filters::Renames,
    ) -> syn::Result<super::MappingPaths> {
        let named_fields = syn::parse_quote!({ #[filter(sw)] username: String });
        super::generate_paths::<DefaultConventions>(
            &parsed(&named_fields),
            mirrors.map(|path| syn::parse_str(path).unwrap()),
            &Integrations::default(),
            renames,
            None,
        )
    }

    // A rename replaces its type's conventions name; the others keep theirs.
    #[test]
    fn test_renames_replace_the_conventions_names() {
        let renames = crate::parse_top_level_meta::filters::Renames {
            query_params: Some(quote::format_ident!("UserSearchParams")),
            sort_field: Some(quote::format_ident!("UserSortKey")),
            ..Default::default()
        };

        let names = with_renames(Some("UserModel"), &renames).unwrap().names;

        assert_eq!(names.query_params, "UserSearchParams");
        assert_eq!(names.sort_field, "UserSortKey");
        assert_eq!(names.sort_details, "UserSortDetailsDto");
    }

    // A root has no query parameters and a mirror no query model to rename.
    #[test]
    fn test_renames_must_name_a_type_of_the_role() {
        let query_params = crate::parse_top_level_meta::filters::Renames {
            query_params: Some(quote::format_ident!("UserSearchParams")),
            ..Default::default()
        };
        let query = crate::parse_top_level_meta::filters::Renames {
            query: Some(quote::format_ident!("UserSearch")),
            ..Default::default()
        };

        assert!(with_renames(None, &query_params).is_err());
        assert!(with_renames(Some("UserModel"), &query).is_err());
        assert!(with_renames(None, &query).is_ok());
    }

    // Two generated types, or one and the struct, cannot share a name.
    #[test]
    fn test_renames_cannot_clash() {
        let same = crate::parse_top_level_meta::filters::Renames {
            sort_field: Some(quote::format_ident!("UserSort")),
            sort_details: Some(quote::format_ident!("UserSort")),
            ..Default::default()
        };
        let struct_name = crate::parse_top_level_meta::filters::Renames {
            filter_options: Some(quote::format_ident!("UserDto")),
            ..Default::default()
        };

        assert!(with_renames(Some("UserModel"), &same).is_err());
        assert!(with_renames(Some("UserModel"), &struct_name).is_err());
    }

    // A struct mirroring itself never reaches a root model.
    #[test]
    fn test_generating_paths_rejects_a_struct_mirroring_itself() {
        let named_fields = syn::parse_quote!({ #[filter(sw)] username: String });

        for path in ["Self", "UserDto"] {
            assert!(
                super::generate_paths::<DefaultConventions>(
                    &parsed(&named_fields),
                    Some(syn::parse_str(path).unwrap()),
                    &Integrations::default(),
                    &crate::parse_top_level_meta::filters::Renames::default(),
                    None,
                )
                .is_err()
            );
        }
    }

    // The flags resolve to the registry's integrations, in its order.
    #[test]
    fn test_resolving_keeps_the_registry_order() {
        let resolved = super::resolve(
            &crate::integrations::BUILT_IN,
            &Integrations::of(&["validator", "serde"]),
        );

        assert_eq!(
            resolved
                .iter()
                .map(|active| active.integration.name())
                .collect::<Vec<_>>(),
            ["serde", "validator"]
        );
    }

    // A root model's code names the core runtime crate.
    #[test]
    fn test_generating_paths_without_a_mirrored_model_is_a_root() {
        let named_fields = syn::parse_quote!({ #[filter(sw)] username: String });

        pretty_assertions::assert_eq!(
            super::generate_paths::<DefaultConventions>(
                &parsed(&named_fields),
                None,
                &Integrations::default(),
                &crate::parse_top_level_meta::filters::Renames::default(),
                None
            )
            .unwrap(),
            super::MappingPaths {
                role: super::Role::Root,
                runtime: syn::parse_quote!(::queryweave),
                names: parsed(&named_fields).idents,
            }
        );
    }

    // `#[queryweave(crate = path)]` overrides the conventions' default.
    #[test]
    fn test_generating_paths_takes_the_given_runtime_crate() {
        let named_fields = syn::parse_quote!({ #[filter(sw)] username: String });

        let mapping_paths = super::generate_paths::<DefaultConventions>(
            &parsed(&named_fields),
            None,
            &Integrations::default(),
            &crate::parse_top_level_meta::filters::Renames::default(),
            Some(syn::parse_quote!(my::runtime)),
        )
        .unwrap();

        pretty_assertions::assert_eq!(mapping_paths.runtime, syn::parse_quote!(my::runtime));
    }

    // Each renamed field of a root model is reported.
    #[test]
    fn test_generating_paths_rejects_equivalent_fields_on_a_root() {
        let named_fields = syn::parse_quote!({
            #[filter(sw)]
            #[equivalent_field(name)]
            username: String,
            #[filter(eq)]
            #[equivalent_field(mail)]
            email: String,
        });

        let error = super::generate_paths::<DefaultConventions>(
            &parsed(&named_fields),
            None,
            &Integrations::default(),
            &crate::parse_top_level_meta::filters::Renames::default(),
            None,
        )
        .unwrap_err();

        assert_eq!(error.into_iter().count(), 2);
    }
}
