/// The intermediary representation that will be turned back into Rust code.
#[derive(Debug, PartialEq, Eq)]
pub struct IntermediaryRepresentation<'a> {
    pub ident: &'a syn::Ident,
    /// Every field, with its type.
    pub fields: Vec<(syn::Ident, &'a syn::Type)>,
}

/// Lower the model of a `MergePatchNullableFields` derive.
#[must_use]
pub fn lower(
    model: crate::analyze::merge_patch_nullable_fields::Model<'_>,
) -> IntermediaryRepresentation<'_> {
    IntermediaryRepresentation {
        ident: model.ident,
        fields: model.fields,
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_lowering_into_ir() {
        let ident = &quote::format_ident!("MergePatchUserModel");
        let ty: syn::Type = syn::parse_quote!(Patch<String>);
        let model = crate::analyze::merge_patch_nullable_fields::Model {
            ident,
            fields: vec![(quote::format_ident!("code"), &ty)],
        };

        let intermediary_representation = super::lower(model);
        let expected_intermediary_representation = super::IntermediaryRepresentation {
            ident,
            fields: vec![(quote::format_ident!("code"), &ty)],
        };

        pretty_assertions::assert_eq!(
            expected_intermediary_representation,
            intermediary_representation
        );
    }
}
