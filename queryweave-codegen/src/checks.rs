//! Compile-time assertions for generated code.
//!
//! When a derive maps names from one type onto another (a DTO field onto its
//! domain model's, a domain field onto an entity's), a name that does not
//! exist on the other side must be a compile error at the name, not a runtime
//! surprise. These helpers emit code that costs nothing at runtime and lets the
//! compiler check it.

use quote::{quote, quote_spanned};

/// Emit a compile-time assertion that each of `fields` is a field of `target`.
///
/// The emitted item is an anonymous `const` holding a never-called closure
/// that borrows each field, so a missing field is reported by rustc as
/// "no field `x` on type `Target`", spanned to that field's ident (pass the
/// ident from the derive input, e.g. the DTO field or its mapping attribute,
/// so the error points there).
///
/// `target` must name a concrete type (with any generic arguments spelled
/// out) whose fields are visible from the call site. Emits nothing when
/// `fields` is empty.
#[must_use]
pub fn assert_fields_exist<'a>(
    target: &syn::Path,
    fields: impl IntoIterator<Item = &'a syn::Ident>,
) -> proc_macro2::TokenStream {
    let borrows = fields
        .into_iter()
        .map(|field| quote_spanned!(field.span()=> let _ = &value.#field;))
        .collect::<Vec<_>>();

    if borrows.is_empty() {
        return proc_macro2::TokenStream::new();
    }

    // Taking `target` by value (the closure is never called) makes rustc
    // name the type itself, not `&Target`, in a missing-field error.
    quote! {
        const _: fn(#target) = |value| {
            #(#borrows)*
        };
    }
}

#[cfg(test)]
mod tests {
    use super::assert_fields_exist;
    use pretty_assertions::assert_eq;

    fn path(source: &str) -> syn::Path {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn one_borrow_per_field_on_the_target() {
        let fields = [
            quote::format_ident!("number"),
            quote::format_ident!("client"),
        ];
        let assertion = assert_fields_exist(&path("domain::MergePatchOrderModel"), &fields);
        let file = syn::parse2::<syn::File>(assertion).unwrap();

        assert_eq!(
            prettyplease::unparse(&file),
            "const _: fn(domain::MergePatchOrderModel) = |value| {\n    let _ = &value.number;\n    let _ = &value.client;\n};\n"
        );
    }

    #[test]
    fn nothing_is_emitted_without_fields() {
        assert!(assert_fields_exist(&path("Model"), &[]).is_empty());
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_assert_fields_exist() {
        use crate::checks::assert_fields_exist;
        let target = syn::parse_str::<syn::Path>("domain::MergePatchOrderModel").unwrap();
        let fields = [
            quote::format_ident!("number"),
            quote::format_ident!("client"),
        ];

        let assertion = assert_fields_exist(&target, &fields).to_string();

        assert!(assertion.contains("value . number"));
        assert!(assertion.contains("value . client"));
    }
}
