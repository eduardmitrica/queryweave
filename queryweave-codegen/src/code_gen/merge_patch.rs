use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

/// Generate the `MergePatch` code.
#[must_use]
pub fn code_gen(
    ir: crate::lower::merge_patch::IntermediaryRepresentation<'_>,
    mapping_paths: crate::mapping::merge_patch::MappingPaths,
) -> proc_macro2::TokenStream {
    let runtime = &mapping_paths.runtime;
    let crate::lower::merge_patch::IntermediaryRepresentation {
        ident,
        field_idents,
    } = ir;
    let equivalent = mapping_paths.equivalent;

    // The patch struct must be a declared merge patch model (deriving
    // `MergePatchNullableFields`), whose derive checks its fields: each an
    // `Option<T>` or a `Patch<T>`, never an `Option<Option<T>>`. Spanned to the
    // path, so a patch struct that is not one is reported there.
    let patch_runtime = crate::utils::respanned(runtime, equivalent.span());
    let patch_check = quote_spanned! {equivalent.span()=>
        const _: fn() = || {
            fn merge_patch_model<T: #patch_runtime::MergePatchNullableFields>() {}
            merge_patch_model::<#equivalent>();
        };
    };

    let field_names: Vec<String> = field_idents
        .iter()
        .map(std::string::ToString::to_string)
        .collect();

    // Collect each changed field's raw value as a database-agnostic
    // `FieldValue`. `RawValue` resolves it from the field's actual type
    // (scalar, value object, optional or not) and `IntoValue` makes it a
    // `queryweave::Value`; spanned to the field, so a type that cannot be
    // stored is reported there.
    let value_statements = field_idents
        .iter()
        .zip(&field_names)
        .map(|(field_ident, field_name)| {
            let value = quote_spanned! {field_ident.span()=>
                #runtime::IntoValue::into_value(#runtime::RawValue::to_raw(&self.#field_ident))
            };

            quote! {
                if diff.contains(#field_name) {
                    values.push(#runtime::FieldValue {
                        field: #field_name,
                        value: #value,
                    });
                }
            }
        })
        .collect::<Vec<proc_macro2::TokenStream>>();
    // With no patchable fields there is nothing to collect; `let mut` and
    // `diff` would then be unused.
    let changed_values_body = if value_statements.is_empty() {
        quote! {
            let _ = diff;
            Vec::new()
        }
    } else {
        quote! {
            let mut values = Vec::new();
            #(#value_statements)*
            values
        }
    };

    // `Some` replaces the field, `None` leaves it untouched. `Into` covers
    // both a patch field of the model's own type (identity) and a value
    // object wrapping it. Spanned to the model field, so a patch field that
    // is missing or of the wrong type is reported there.
    let mut merge_statements = field_idents
        .iter()
        .map(|field_ident| {
            // Through `PatchField`, so the patch field's type (`Option<P>`, or
            // `Patch<P>` for a field that may be cleared) needs no
            // classification here.
            let runtime = crate::utils::respanned(runtime, field_ident.span());
            quote_spanned! {field_ident.span()=>
                #runtime::PatchField::apply_to(merge_patch.#field_ident, &mut self.#field_ident);
            }
        })
        .collect::<Vec<proc_macro2::TokenStream>>();
    if merge_statements.is_empty() {
        merge_statements.push(quote!(let _ = merge_patch;));
    }

    let expanded = quote! {
        impl #runtime::ChangedFields for #ident {
            /// Iterate through all fields, excluding fields marked as #[merge_patch(skip)]
            /// and return a change set (a hash-set consisting of the field names
            /// of the fields that were changed).
            fn modified_fields(&self, previous: &Self) -> #runtime::ChangeSet {
                let mut fields_that_were_changed = #runtime::ChangeSet::default();
                #(
                    if self.#field_idents != previous.#field_idents {
                        _ = fields_that_were_changed.insert(#field_names);
                    }
                )*

                fields_that_were_changed
            }
        }

        /// The domain fields a merge patch may write (those not marked
        /// `#[merge_patch(skip)]`, i.e. what `changed_values` can return),
        /// for the persistence layer to map onto columns.
        impl #runtime::PersistedFields for #ident {
            const FIELD_NAMES: &'static [&'static str] = &[#(#field_names),*];
        }

        impl #ident {
            /// Merge a model with a patch resulting in a modified model.
            pub fn merge(&mut self, merge_patch: #equivalent) {
                #(#merge_statements)*
            }

            /// The changed fields (per `diff`, see `ChangedFields::modified_fields`)
            /// with their current raw values, as database-agnostic field/value
            /// pairs for the persistence layer to write.
            #[must_use]
            pub fn changed_values(&self, diff: &#runtime::ChangeSet) -> #runtime::FieldValues<Self> {
                #runtime::__private::model_data({ #changed_values_body })
            }
        }

        #patch_check
    };

    expanded
}

#[cfg(test)]
mod tests {

    /// Pretty-print the generated items, which also asserts they form a valid
    /// Rust file.
    fn pretty(token_stream: proc_macro2::TokenStream) -> String {
        prettyplease::unparse(&syn::parse2::<syn::File>(token_stream).unwrap())
    }

    #[test]
    fn test_code_gen() {
        let field_idents = vec![
            quote::format_ident!("client"),
            quote::format_ident!("capacity"),
        ];
        let intermediary_representation = crate::lower::merge_patch::IntermediaryRepresentation {
            field_idents,
            ident: &quote::format_ident!("OrderModel"),
        };

        let mapping_paths = crate::mapping::merge_patch::MappingPaths {
            equivalent: syn::parse_str::<syn::Path>("MergePatchOrderModel").unwrap(),
            runtime: syn::parse_quote!(::queryweave),
        };

        insta::assert_snapshot!(pretty(super::code_gen(
            intermediary_representation,
            mapping_paths
        )));
    }
}
