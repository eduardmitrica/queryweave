/// The intermediary representation that will be turned back into Rust code.
#[derive(Debug, PartialEq, Eq)]
pub struct IntermediaryRepresentation<'a> {
    pub ident: &'a syn::Ident,

    /// The patchable fields.
    pub field_idents: Vec<syn::Ident>,
    /// The type of each patchable field (same index in `field_idents`).
    pub field_types: Vec<syn::Type>,
}

/// Lower the model of a `MergePatch` derive on the struct `ident`.
#[must_use]
pub fn lower(
    model: crate::analyze::merge_patch::Model,
    ident: &syn::Ident,
) -> IntermediaryRepresentation<'_> {
    IntermediaryRepresentation {
        ident,
        field_idents: model.field_idents,
        field_types: model.field_types,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_lowering_into_ir() {
        let ident = quote::format_ident!("OrderModel");
        let field_idents = vec![
            quote::format_ident!("client"),
            quote::format_ident!("capacity"),
        ];
        let field_types = vec![
            syn::parse_quote!(Option<String>),
            syn::parse_quote!(OrderCapacity),
        ];
        let model = crate::analyze::merge_patch::Model {
            field_idents: field_idents.clone(),
            field_types: field_types.clone(),
        };

        pretty_assertions::assert_eq!(
            super::lower(model, &ident),
            super::IntermediaryRepresentation {
                ident: &ident,
                field_idents,
                field_types,
            }
        );
    }
}
