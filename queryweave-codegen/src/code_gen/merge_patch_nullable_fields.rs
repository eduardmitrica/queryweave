use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// Generate the `MergePatchNullableFields` code.
#[must_use]
pub fn code_gen(
    ir: crate::lower::merge_patch_nullable_fields::IntermediaryRepresentation<'_>,
    mapping_paths: &crate::mapping::merge_patch_nullable_fields::MappingPaths,
) -> proc_macro2::TokenStream {
    let runtime = &mapping_paths.runtime;
    let crate::lower::merge_patch_nullable_fields::IntermediaryRepresentation { ident, fields } =
        ir;

    // Each field answers through its type's `PatchKind`, spanned to the type,
    // so a field that is neither an `Option` nor a `Patch` is reported there.
    let arms = fields.iter().map(|(field, ty)| {
        let name = syn::LitStr::new(&field.to_string(), field.span());
        let field_runtime = crate::utils::respanned(runtime, ty.span());
        quote_spanned! {ty.span()=>
            #name => <#ty as #field_runtime::PatchKind>::NULLABLE,
        }
    });

    quote! {
        impl #runtime::MergePatchNullableFields for #ident {
            /// Whether the field named `field` may be set to `null`: its
            /// type is a `Patch`.
            fn is_nullable(field: &str) -> bool {
                match field {
                    #(#arms)*
                    _ => false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    // Each field answers through its type's `PatchKind`.
    #[test]
    fn test_code_gen() {
        let patch: syn::Type = syn::parse_quote!(Patch<String>);
        let option: syn::Type = syn::parse_quote!(Option<i32>);
        let intermediary_representation =
            crate::lower::merge_patch_nullable_fields::IntermediaryRepresentation {
                ident: &quote::format_ident!("MergePatchUserModel"),
                fields: vec![
                    (quote::format_ident!("code"), &patch),
                    (quote::format_ident!("count"), &option),
                ],
            };

        let token_stream = super::code_gen(
            intermediary_representation,
            &crate::mapping::merge_patch_nullable_fields::MappingPaths {
                runtime: syn::parse_quote!(::queryweave),
            },
        );
        let code = token_stream.to_string();

        assert!(syn::parse2::<syn::ItemImpl>(token_stream).is_ok());
        assert!(code.contains(
            "\"code\" => < Patch < String > as :: queryweave :: PatchKind > :: NULLABLE"
        ));
        assert!(
            code.contains(
                "\"count\" => < Option < i32 > as :: queryweave :: PatchKind > :: NULLABLE"
            )
        );
    }
}
