/// The intermediary representation that will be turned back into Rust code.
#[derive(Debug, PartialEq, Eq)]
pub struct IntermediaryRepresentation<'a> {
    pub ident: &'a syn::Ident,
    pub field_idents: Vec<syn::Ident>,
    /// The ident each DTO field (same index in `field_idents`) has on the
    /// domain merge patch model.
    pub equivalent_field_idents: Vec<syn::Ident>,
}

/// Lower the model of a `ValidateNonNullable` derive on the struct `ident`.
#[must_use]
pub fn lower(
    model: crate::analyze::validate_non_nullable::Model,
    ident: &syn::Ident,
) -> IntermediaryRepresentation<'_> {
    IntermediaryRepresentation {
        ident,
        field_idents: model.field_idents,
        equivalent_field_idents: model.equivalent_field_idents,
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_lowering_into_ir() {
        let ident = &quote::format_ident!("MergePatchOrderDto");
        let field_idents = vec![
            quote::format_ident!("value"),
            quote::format_ident!("order_number"),
        ];
        let equivalent_field_idents = vec![
            quote::format_ident!("value"),
            quote::format_ident!("number"),
        ];
        let model = crate::analyze::validate_non_nullable::Model {
            field_idents: field_idents.clone(),
            equivalent_field_idents: equivalent_field_idents.clone(),
        };

        pretty_assertions::assert_eq!(
            super::lower(model, ident),
            super::IntermediaryRepresentation {
                ident,
                field_idents,
                equivalent_field_idents,
            }
        );
    }
}
