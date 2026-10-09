/// The result of interpreting the parser's output.
#[derive(Debug, PartialEq, Eq, derive_new::new)]
pub struct Model<'a> {
    pub ident: &'a syn::Ident,
    /// Every field, with its type: whether it may be set to `null` is its
    /// type's (`queryweave::PatchKind::NULLABLE`), decided by the compiler.
    #[new(default)]
    pub fields: Vec<(syn::Ident, &'a syn::Type)>,
}

/// Collect the patch fields of a merge patch model (those not marked
/// `#[merge_patch(skip)]`, e.g. an id) and their types.
///
/// A field is nullable when its type is a `queryweave::Patch<T>`, which the
/// generated code asks the field's type (through `queryweave::PatchKind`),
/// rather than this step recognising the type by its name or shape.
///
/// # Errors
///
/// Returns the errors of malformed `#[merge_patch(...)]` attributes, and of
/// `Option<Option<T>>` fields (a field that may be cleared is a
/// `queryweave::Patch<T>`).
pub fn analyze<'a>(
    parsed: &'a crate::parse::merge_patch_nullable_fields::Parsed<'_>,
) -> syn::Result<Model<'a>> {
    let mut model = Model::new(parsed.ident);
    let mut errors = crate::errors::Errors::default();

    for field in &parsed.named_fields.named {
        let Some(field_ident) = &field.ident else {
            continue;
        };
        if errors.check(crate::attributes::is_skipped(field, "merge_patch")) == Some(false)
            && errors
                .check(crate::attributes::reject_option_of_option(field))
                .is_some()
        {
            model.fields.push((field_ident.clone(), &field.ty));
        }
    }

    errors.finish(model)
}

#[cfg(test)]
mod tests {

    /// The fields the analysis keeps, by name.
    fn field_names(named_fields_input: proc_macro2::TokenStream) -> syn::Result<Vec<String>> {
        let ident = &quote::format_ident!("MergePatchUserModel");
        let named_fields = &syn::parse2(named_fields_input).unwrap();
        let parsed = crate::parse::merge_patch_nullable_fields::Parsed {
            ident,
            named_fields,
        };

        Ok(super::analyze(&parsed)?
            .fields
            .into_iter()
            .map(|(field, _)| field.to_string())
            .collect())
    }

    // Every field is kept, whatever its type (its nullability is its type's
    // to say), in declaration order, except the skipped ones.
    #[test]
    fn test_every_field_but_the_skipped_ones_is_kept() {
        let fields = field_names(quote::quote! {
            {
                #[merge_patch(skip)]
                id: UserId,
                number: Patch<String>,
                capacity: Option<i32>,
                client: Patch<UserClient<String>>,
            }
        })
        .unwrap();

        pretty_assertions::assert_eq!(fields, ["number", "capacity", "client"]);
    }

    #[test]
    fn test_a_malformed_skip_is_rejected() {
        for attribute in [
            quote::quote!(#[merge_patch()]),
            quote::quote!(#[merge_patch(keep)]),
        ] {
            assert!(field_names(quote::quote!({ #attribute id: UserId })).is_err());
        }
    }

    // A field that may be cleared is a `Patch<T>`; `Option<Option<T>>` is
    // rejected rather than read as a field that cannot be cleared.
    #[test]
    fn test_option_of_option_is_rejected() {
        assert!(field_names(quote::quote!({ code: Option<Option<String>> })).is_err());
    }
}
