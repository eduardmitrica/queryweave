//! Struct and field attributes shared by several derives.

use syn::spanned::Spanned;

/// What an option of a struct attribute `#[name(...)]` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    /// A path: `key = path`.
    Path,
    /// Nothing: a bare `key`, set by being written.
    Flag,
}

/// What [`read_option`] found for its key.
enum Found {
    Path(syn::Path),
    Flag(syn::Ident),
}

/// Walk the options of every `#[name(...)]` attribute, checking each against
/// `options` (an unknown, repeated or malformed one is an error, whichever
/// option the caller reads), and return the value of `key`.
fn read_option(
    attributes: &[syn::Attribute],
    name: &str,
    key: &str,
    options: &[(&str, OptionKind)],
    help: &str,
) -> syn::Result<Option<Found>> {
    let mut found = None;
    let names = options
        .iter()
        .map(|(option, _)| *option)
        .collect::<Vec<_>>()
        .join(", ");

    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident(name))
    {
        let usage = format!("Provide options as #[{name}({names})]");
        let Ok(list) = attribute.meta.require_list() else {
            let message = format!("The #[{name}(...)] attribute takes options");
            return crate::errors::abort_spanned(attribute, &message, &usage);
        };
        if list.tokens.is_empty() {
            let message = format!("The #[{name}(...)] attribute cannot be left empty");
            return crate::errors::abort_spanned(list, &message, &usage);
        }

        attribute.parse_nested_meta(|meta| {
            let option = meta
                .path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default();
            let Some(&(_, kind)) = options.iter().find(|(known, _)| *known == option) else {
                let written = quote::ToTokens::to_token_stream(&meta.path).to_string();
                let message = format!("Unknown #[{name}(...)] option `{written}`");
                let help = format!("Use one of: {names}");
                return Err(meta.error(crate::errors::message(&message, &help)));
            };

            let value = match kind {
                OptionKind::Path => {
                    // Reported at the option, rather than at whatever follows it.
                    let value = meta.value().map_err(|_| {
                        let message =
                            format!("The `{option}` option of #[{name}(...)] takes a value");
                        let help = format!("Write the option as #[{name}({option} = ...)]");
                        syn::Error::new_spanned(&meta.path, crate::errors::message(&message, &help))
                    })?;
                    if option != key {
                        // Another option of the attribute, read by its own call.
                        return value.parse::<syn::Expr>().map(drop);
                    }
                    let path = value.parse::<syn::Path>().map_err(|error| {
                        let message = format!("The `{key}` option of #[{name}(...)] takes a path");
                        syn::Error::new(error.span(), crate::errors::message(&message, help))
                    })?;
                    Found::Path(path)
                },
                OptionKind::Flag => {
                    if meta.input.peek(syn::Token![=]) {
                        let message =
                            format!("The `{option}` option of #[{name}(...)] takes no value");
                        let help = format!("Write the option alone: #[{name}({option})]");
                        return Err(syn::Error::new_spanned(
                            &meta.path,
                            crate::errors::message(&message, &help),
                        ));
                    }
                    if option != key {
                        return Ok(());
                    }
                    // A known option is a single identifier.
                    let Some(ident) = meta.path.get_ident() else {
                        return Ok(());
                    };
                    Found::Flag(ident.clone())
                },
            };

            if found.is_some() {
                let message = format!("Duplicate `{key}` option of #[{name}(...)]");
                let help = format!("Give `{key}` only once");
                let span = match &value {
                    Found::Path(path) => syn::spanned::Spanned::span(path),
                    Found::Flag(ident) => ident.span(),
                };
                return Err(syn::Error::new(
                    span,
                    crate::errors::message(&message, &help),
                ));
            }
            found = Some(value);

            Ok(())
        })?;
    }

    Ok(found)
}

/// The path given to the option `key` of the struct attribute
/// `#[name(key = path, ...)]`, if any.
///
/// E.g. `#[filters(mirrors = domain::OrderModel)]`. The attribute is optional
/// and may be repeated, but each option may be given only once; `options`
/// lists every option the attribute accepts, with what it takes, so an
/// unknown or malformed one is an error rather than silently ignored. `help`
/// tells the user what the path names, for a malformed option.
///
/// # Errors
///
/// Returns a spanned error when the attribute is empty or not a list, when an
/// option is unknown, repeated or malformed (`key = value` for a path, a bare
/// `key` for a flag), or when the value of `key` is not a path.
pub fn path_option(
    attributes: &[syn::Attribute],
    name: &str,
    key: &str,
    options: &[(&str, OptionKind)],
    help: &str,
) -> syn::Result<Option<syn::Path>> {
    Ok(match read_option(attributes, name, key, options, help)? {
        Some(Found::Path(path)) => Some(path),
        Some(Found::Flag(_)) | None => None,
    })
}

/// The name given to the option `key` of the struct attribute
/// `#[name(key = Name, ...)]`, if any: [`path_option`], for an option naming
/// an item rather than linking to one (e.g. a generated type's name).
///
/// # Errors
///
/// Returns the errors of [`path_option`], and an error at the value when it
/// is a path rather than a single name.
pub fn ident_option(
    attributes: &[syn::Attribute],
    name: &str,
    key: &str,
    options: &[(&str, OptionKind)],
    help: &str,
) -> syn::Result<Option<syn::Ident>> {
    path_option(attributes, name, key, options, help)?
        .map(|path| {
            path.get_ident().cloned().ok_or_else(|| {
                let message =
                    format!("The `{key}` option of #[{name}(...)] takes a name, not a path");
                syn::Error::new_spanned(&path, crate::errors::message(&message, help))
            })
        })
        .transpose()
}

/// The flag `key` of the struct attribute `#[name(key, ...)]`, if written: its
/// ident, so an error about it can point at it.
///
/// E.g. `#[filters(mirrors = domain::OrderModel, serde)]`. See
/// [`path_option`] for `options`.
///
/// # Errors
///
/// See [`path_option`].
pub fn flag_option(
    attributes: &[syn::Attribute],
    name: &str,
    key: &str,
    options: &[(&str, OptionKind)],
) -> syn::Result<Option<syn::Ident>> {
    Ok(match read_option(attributes, name, key, options, "")? {
        Some(Found::Flag(ident)) => Some(ident),
        Some(Found::Path(_)) | None => None,
    })
}

/// The path given to the required option `key` of the struct attribute
/// `#[name(key = path, ...)]`: [`path_option`], for an option the derive
/// cannot do without.
///
/// E.g. `#[validate_non_nullable(mirrors = MergePatchOrderModel)]`.
///
/// # Errors
///
/// Returns the errors of [`path_option`], and an error at the derive when the
/// option is missing.
pub fn required_path_option(
    attributes: &[syn::Attribute],
    name: &str,
    key: &str,
    options: &[(&str, OptionKind)],
    help: &str,
) -> syn::Result<syn::Path> {
    require_option(
        path_option(attributes, name, key, options, help)?,
        name,
        key,
        help,
    )
}

/// The path `path` read for the required option `key` of `#[name(...)]`,
/// or an error at the derive when it is missing.
///
/// [`required_path_option`]'s check, for a derive reading several required
/// options of one attribute with [`path_option`], to report every missing
/// one together.
///
/// # Errors
///
/// Returns an error at the derive when `path` is `None`.
pub fn require_option(
    path: Option<syn::Path>,
    name: &str,
    key: &str,
    help: &str,
) -> syn::Result<syn::Path> {
    path.map_or_else(
        || {
            let message = format!("No #[{name}({key} = ...)] attribute provided");
            crate::errors::abort_call_site(&message, help)
        },
        Ok,
    )
}

/// Reject a link option `#[name(key = path)]` whose path names the struct
/// itself (`Self`, or its own name without a module), e.g. a struct mirroring
/// itself: the chain of links would never end.
///
/// A path with a module (`api::UserDto` on `grpc::UserDto`) may name another
/// struct of the same name, so it is left to the compiler.
///
/// # Errors
///
/// Returns an error at the path when it names `own`.
pub fn reject_self_link(
    path: &syn::Path,
    own: &syn::Ident,
    name: &str,
    key: &str,
) -> syn::Result<()> {
    let segments = path
        .segments
        .iter()
        .map(|segment| &segment.ident)
        .skip_while(|ident| path.leading_colon.is_none() && *ident == "self")
        .collect::<Vec<_>>();
    let is_self = match segments.as_slice() {
        [ident] => *ident == "Self" || *ident == own,
        _ => false,
    };
    if is_self {
        let message = format!("`{own}` cannot link to itself with #[{name}({key} = ...)]");
        let help = format!("Name another struct, which `{own}` {key}");
        return Err(syn::Error::new_spanned(
            path,
            crate::errors::message(&message, &help),
        ));
    }
    Ok(())
}

#[cfg(feature = "merge-patch")]
/// Reject a merge patch field written `Option<Option<T>>`: merge patches
/// express "absent, `null` or a value" only with `queryweave::Patch<T>`. Only
/// the field's `Option` shape is looked at.
///
/// Without this, such a field would still compile, as an `Option` field that
/// cannot be cleared, silently ignoring the `null` it was meant to carry.
///
/// # Errors
///
/// Returns an error at the field's type when it is an `Option<Option<T>>`.
pub fn reject_option_of_option(field: &syn::Field) -> syn::Result<()> {
    let is_option_of_option = crate::utils::unwrap_option(&field.ty)
        .and_then(crate::utils::unwrap_option)
        .is_some();
    if is_option_of_option {
        return crate::errors::abort_spanned(
            &field.ty,
            "Merge patches do not support `Option<Option<T>>`",
            "Write `queryweave::Patch<T>` for a field that may be cleared (absent, `null` or a value), or `Option<T>` for one that may not (absent or a value)",
        );
    }
    Ok(())
}

#[cfg(feature = "merge-patch")]
/// Whether `field` is marked `#[name(skip)]` (e.g. `#[merge_patch(skip)]`,
/// a field a derive leaves out).
///
/// # Errors
///
/// Returns an error at the attribute when it is not a list, is empty, or
/// holds anything but `skip`.
pub fn is_skipped(field: &syn::Field, name: &str) -> syn::Result<bool> {
    let mut skipped = false;
    for attribute in field
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident(name))
    {
        let list = attribute.meta.require_list()?;
        if list.tokens.is_empty() {
            let message = "The list of arguments of the attribute cannot be left empty";
            let help = format!("Provide the skip argument: #[{name}(skip)]");
            return crate::errors::abort_spanned(list, message, &help);
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skipped = true;
                return Ok(());
            }
            let message = format!("Only skip is available inside the {name} attribute");
            let help = format!("Either remove #[{name}(...)] or correct it to be #[{name}(skip)]");
            Err(meta.error(crate::errors::message(&message, &help)))
        })?;
    }
    Ok(skipped)
}

/// The path given by `#[queryweave(crate = path)]`, if any: where the runtime
/// crate the generated code names lives, for a crate renaming or re-exporting
/// it. Every derive accepts it.
///
/// # Errors
///
/// See [`path_option`].
pub fn crate_option(attributes: &[syn::Attribute]) -> syn::Result<Option<syn::Path>> {
    path_option(
        attributes,
        "queryweave",
        "crate",
        &[("crate", OptionKind::Path)],
        "Name where the runtime crate lives: #[queryweave(crate = path)]",
    )
}

/// The name `field` has on the equivalent type a derive maps it onto (e.g. a
/// DTO field onto its domain model's): the name in `#[equivalent_field(name)]`,
/// or the field's own name without the attribute.
///
/// # Errors
///
/// See [`mapped_field_name`].
pub fn equivalent_field_name(field: &syn::Field) -> syn::Result<syn::Ident> {
    mapped_field_name(field, "equivalent_field")
}

/// The name `field` maps onto on another type, per a renaming attribute
/// `#[attribute(name)]` (e.g. `equivalent_field`, `domain_field`), or the
/// field's own name without the attribute.
///
/// The returned ident carries the span of what the user wrote (the name inside
/// the attribute, or the field's ident), so generated code naming it reports a
/// missing field there.
///
/// # Errors
///
/// Returns a spanned error when the field has more than one such attribute,
/// or when the attribute is not a list holding exactly one identifier.
pub fn mapped_field_name(field: &syn::Field, attribute: &str) -> syn::Result<syn::Ident> {
    let mut attributes = field
        .attrs
        .iter()
        .filter(|candidate| candidate.path().is_ident(attribute));

    let Some(found) = attributes.next() else {
        return field.ident.clone().ok_or_else(|| {
            syn::Error::new(
                field.span(),
                crate::errors::message(
                    "Cannot map an unnamed field onto another type's field",
                    "Use a struct with named fields",
                ),
            )
        });
    };

    if let Some(duplicate) = attributes.next() {
        let message = format!("A field can only have one #[{attribute}(...)] attribute");
        let help = format!("Keep a single #[{attribute}(name)] on the field");
        return crate::errors::abort_spanned(duplicate, &message, &help);
    }

    let help = format!("Name the field it maps to: #[{attribute}(name)]");
    let Ok(list) = found.meta.require_list() else {
        let message =
            format!("The #[{attribute}(...)] attribute takes the name of the field it maps to");
        return crate::errors::abort_spanned(found, &message, &help);
    };

    list.parse_args::<syn::Ident>().or_else(|_| {
        let message = format!("The #[{attribute}(...)] attribute takes exactly one field name");
        crate::errors::abort_spanned(list, &message, &help)
    })
}

#[cfg(test)]
mod tests {
    use super::{equivalent_field_name, mapped_field_name, path_option};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    fn struct_attributes(item: proc_macro2::TokenStream) -> Vec<syn::Attribute> {
        syn::parse2::<syn::DeriveInput>(item).unwrap().attrs
    }

    // The generic form reads the attribute it is given, and only that one.
    #[test]
    fn the_given_attribute_names_the_mapped_field() {
        let field: syn::Field = syn::parse_quote!(
            #[equivalent_field(number)] #[domain_field(delivery_date)] delivered_at: String
        );

        assert_eq!(
            mapped_field_name(&field, "domain_field").unwrap(),
            "delivery_date"
        );
        assert_eq!(
            mapped_field_name(&field, "equivalent_field").unwrap(),
            "number"
        );
        assert_eq!(
            mapped_field_name(&field, "other_field").unwrap(),
            "delivered_at"
        );
    }

    // Errors name the attribute that was malformed.
    #[test]
    fn an_error_names_the_attribute() {
        let field: syn::Field = syn::parse_quote!(#[domain_field()] delivered_at: String);

        let error = mapped_field_name(&field, "domain_field").unwrap_err();

        assert!(
            error
                .to_string()
                .starts_with("The #[domain_field(...)] attribute takes exactly one field name")
        );
    }

    #[test]
    fn a_field_without_the_attribute_keeps_its_own_name() {
        let field: syn::Field = syn::parse_quote!(#[serde(default)] client: String);

        assert_eq!(equivalent_field_name(&field).unwrap(), "client");
    }

    #[test]
    fn the_attribute_names_the_equivalent_field() {
        let field: syn::Field =
            syn::parse_quote!(#[serde(default)] #[equivalent_field(number)] order_number: String);

        assert_eq!(equivalent_field_name(&field).unwrap(), "number");
    }

    #[rstest]
    // Not a list.
    #[case(syn::parse_quote!(#[equivalent_field] order_number: String))]
    #[case(syn::parse_quote!(#[equivalent_field = "number"] order_number: String))]
    // An empty list, a non-identifier, or more than one name.
    #[case(syn::parse_quote!(#[equivalent_field()] order_number: String))]
    #[case(syn::parse_quote!(#[equivalent_field("number")] order_number: String))]
    #[case(syn::parse_quote!(#[equivalent_field(number, client)] order_number: String))]
    // More than one attribute.
    #[case(syn::parse_quote!(#[equivalent_field(number)] #[equivalent_field(client)] order_number: String))]
    fn a_malformed_attribute_is_rejected(#[case] field: syn::Field) {
        assert!(equivalent_field_name(&field).is_err());
    }

    const OPTIONS: [(&str, super::OptionKind); 3] = [
        ("mirrors", super::OptionKind::Path),
        ("other", super::OptionKind::Path),
        ("serde", super::OptionKind::Flag),
    ];

    fn mirrors(item: proc_macro2::TokenStream) -> syn::Result<Option<syn::Path>> {
        path_option(
            &struct_attributes(item),
            "filters",
            "mirrors",
            &OPTIONS,
            "help",
        )
    }

    fn serde(item: proc_macro2::TokenStream) -> syn::Result<Option<syn::Ident>> {
        super::flag_option(&struct_attributes(item), "filters", "serde", &OPTIONS)
    }

    // A flag is set by being written, next to the other options.
    #[test]
    fn flag_option_reads_a_written_flag() {
        let item = quote::quote! {
            #[filters(mirrors = OrderModel, serde)]
            struct OrderDto;
        };

        assert_eq!(
            serde(item.clone()).unwrap(),
            Some(quote::format_ident!("serde"))
        );
        assert_eq!(mirrors(item).unwrap(), Some(syn::parse_quote!(OrderModel)));
        assert_eq!(
            serde(quote::quote! { #[filters(mirrors = OrderModel)] struct OrderDto; }).unwrap(),
            None
        );
    }

    #[rstest]
    #[case::with_a_value(quote::quote! { #[filters(serde = true)] struct OrderDto; })]
    #[case::repeated(quote::quote! { #[filters(serde, serde)] struct OrderDto; })]
    #[case::path_option_without_value(quote::quote! { #[filters(serde, mirrors)] struct OrderDto; })]
    fn flag_option_rejects_malformed_options(#[case] item: proc_macro2::TokenStream) {
        assert!(serde(item).is_err());
    }

    #[test]
    fn path_option_reads_the_path_of_its_key() {
        assert_eq!(
            mirrors(quote::quote! {
                #[filters(mirrors = domain::OrderModel)]
                struct OrderDto;
            })
            .unwrap(),
            Some(syn::parse_quote!(domain::OrderModel))
        );
    }

    // No attribute, or one giving only other options, leaves the option unset.
    #[test]
    fn path_option_is_none_without_its_key() {
        assert_eq!(mirrors(quote::quote! { struct OrderModel; }).unwrap(), None);
        assert_eq!(
            mirrors(quote::quote! {
                #[filters(other = 1)]
                struct OrderModel;
            })
            .unwrap(),
            None
        );
    }

    // Options may be spread over several attributes.
    #[test]
    fn path_option_reads_across_attributes() {
        assert_eq!(
            mirrors(quote::quote! {
                #[filters(other = 1)]
                #[filters(mirrors = OrderModel)]
                struct OrderDto;
            })
            .unwrap(),
            Some(syn::parse_quote!(OrderModel))
        );
    }

    #[rstest]
    #[case::empty(quote::quote! { #[filters()] struct OrderDto; })]
    #[case::not_a_list(quote::quote! { #[filters] struct OrderDto; })]
    #[case::unknown_option(quote::quote! { #[filters(mirror = OrderModel)] struct OrderDto; })]
    #[case::no_value(quote::quote! { #[filters(mirrors)] struct OrderDto; })]
    #[case::not_a_path(quote::quote! { #[filters(mirrors = "OrderModel")] struct OrderDto; })]
    #[case::repeated(quote::quote! { #[filters(mirrors = A, mirrors = B)] struct OrderDto; })]
    #[case::repeated_across_attributes(quote::quote! {
        #[filters(mirrors = A)]
        #[filters(mirrors = B)]
        struct OrderDto;
    })]
    fn path_option_rejects_malformed_options(#[case] item: proc_macro2::TokenStream) {
        assert!(mirrors(item).is_err());
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_path_option() {
        use crate::attributes::{OptionKind, path_option};
        let options = [("mirrors", OptionKind::Path), ("serde", OptionKind::Flag)];
        let mirror: syn::DeriveInput = syn::parse_quote! {
            #[filters(mirrors = domain::OrderModel, serde)]
            struct OrderDto;
        };
        let root: syn::DeriveInput = syn::parse_quote! {
            struct OrderModel;
        };

        assert_eq!(
            path_option(&mirror.attrs, "filters", "mirrors", &options, "help").unwrap(),
            Some(syn::parse_quote!(domain::OrderModel))
        );
        assert_eq!(
            path_option(&root.attrs, "filters", "mirrors", &options, "help").unwrap(),
            None
        );
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_flag_option() {
        use crate::attributes::{OptionKind, flag_option};
        let options = [("mirrors", OptionKind::Path), ("serde", OptionKind::Flag)];
        let item: syn::DeriveInput = syn::parse_quote! {
            #[filters(mirrors = domain::OrderModel, serde)]
            struct OrderDto;
        };

        assert!(
            flag_option(&item.attrs, "filters", "serde", &options)
                .unwrap()
                .is_some()
        );
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_required_path_option() {
        use crate::attributes::{OptionKind, required_path_option};
        let unlinked: syn::DeriveInput = syn::parse_quote! {
            struct MergePatchOrderDto;
        };

        assert!(
            required_path_option(
                &unlinked.attrs,
                "validate_non_nullable",
                "mirrors",
                &[("mirrors", OptionKind::Path)],
                "help"
            )
            .is_err()
        );
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_require_option() {
        use crate::attributes::require_option;
        assert!(require_option(None, "append_to_query", "filters", "help").is_err());
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_reject_self_link() {
        use crate::attributes::reject_self_link;
        let own = quote::format_ident!("UserDto");

        assert!(reject_self_link(&syn::parse_quote!(Self), &own, "filters", "mirrors").is_err());
        assert!(reject_self_link(&syn::parse_quote!(UserDto), &own, "filters", "mirrors").is_err());
        assert!(
            reject_self_link(&syn::parse_quote!(api::UserDto), &own, "filters", "mirrors").is_ok()
        );
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_crate_option() {
        use crate::attributes::crate_option;
        let item: syn::DeriveInput = syn::parse_quote! {
            #[queryweave(crate = my::runtime)]
            struct OrderModel;
        };

        assert_eq!(
            crate_option(&item.attrs).unwrap(),
            Some(syn::parse_quote!(my::runtime))
        );
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_equivalent_field_name() {
        use crate::attributes::equivalent_field_name;
        let renamed: syn::Field =
            syn::parse_quote!(#[equivalent_field(number)] order_number: String);
        let plain: syn::Field = syn::parse_quote!(client: String);

        assert_eq!(equivalent_field_name(&renamed).unwrap(), "number");
        assert_eq!(equivalent_field_name(&plain).unwrap(), "client");
    }
}
