//! `validator`: validation of the query parameters (`page_size` at least 1,
//! `sort` at most one detail per sortable field, none repeated).

use quote::quote;

/// The `validator` integration.
pub struct Validator;

impl super::Integration for Validator {
    fn name(&self) -> &'static str {
        "validator"
    }

    fn requires(&self) -> &'static [(&'static str, &'static str)] {
        &[(
            "serde",
            "a validation error carries the rejected value, serialized",
        )]
    }

    fn supports_validate_non_nullable(&self) -> bool {
        true
    }

    /// `validate_non_nullable() -> Result<(), validator::ValidationErrors>`:
    /// the struct's `validator::Validate`, then a `non-nullable` error under
    /// each field set to `null` that may not be.
    fn validate_non_nullable(
        &self,
        context: &super::NonNullableContext<'_>,
    ) -> proc_macro2::TokenStream {
        let super::NonNullableContext {
            ident,
            fields,
            flag,
            ..
        } = *context;
        let names = fields
            .iter()
            .map(|field| syn::LitStr::new(&field.to_string(), field.span()));
        let messages = fields.iter().map(|field| {
            format!(
                "{} cannot be set to null",
                crate::utils::uppercase_first_letter(&field.to_string())
            )
        });
        // Spanned to the flag: the DTO must derive `validator::Validate`.
        let validate_check = quote::quote_spanned! {flag.span()=>
            // Never called: an anonymous const, so the check adds no name to
            // the user's type.
            const _: fn() = || {
                fn validator_validate<T: validator::Validate>() {}
                validator_validate::<#ident>();
            };
        };

        quote! {
            #validate_check

            impl #ident {
                /// Validate the struct (`validator::Validate`), then reject each
                /// field set to `null` that may not be (see `rejected_nulls`).
                ///
                /// # Errors
                ///
                /// Returns the validation errors, a `non-nullable` one under each
                /// field rejected.
                pub fn validate_non_nullable(&self) -> Result<(), validator::ValidationErrors> {
                    validator::Validate::validate(&self)?;

                    let mut errors = validator::ValidationErrors::new();
                    for field in self.rejected_nulls() {
                        let message: &'static str = match field {
                            #(#names => #messages,)*
                            _ => "This field cannot be set to null",
                        };
                        errors.add(
                            field,
                            validator::ValidationError::new("non-nullable").with_message(message.into()),
                        );
                    }

                    if errors.is_empty() {
                        Ok(())
                    } else {
                        Err(errors)
                    }
                }
            }
        }
    }

    fn query_params(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(validator::Validate)],
            attributes: quote!(),
        }
    }

    fn page_size(&self, _context: &super::Context<'_>) -> proc_macro2::TokenStream {
        quote!(#[validate(range(min = 1))])
    }

    fn sort(&self, context: &super::Context<'_>) -> proc_macro2::TokenStream {
        let crate::conventions::FiltersNames {
            sort_field,
            sort_details,
            ..
        } = context.names;
        let validate_sort = syn::LitStr::new(
            &format!("{sort_details}::validate_sort"),
            proc_macro2::Span::call_site(),
        );

        quote! {
            #[validate(length(min = 1, max = #sort_field::COUNT))]
            #[validate(custom(function = #validate_sort))]
        }
    }

    /// The custom `validator` function the sortable query params run on
    /// `sort`: an associated function of the sort details type rather than a
    /// free one, so several sortable mirrors can live in one module without
    /// their validators colliding.
    fn sortable_items(&self, context: &super::Context<'_>) -> proc_macro2::TokenStream {
        let crate::conventions::FiltersNames {
            sort_field,
            sort_details,
            ..
        } = context.names;

        quote! {
            impl #sort_details {
                /// Reject sort details that sort on the same field more than once.
                ///
                /// # Errors
                ///
                /// Returns a `sort` validation error when a field is repeated.
                pub fn validate_sort(sort: &[Self]) -> Result<(), validator::ValidationError> {
                    let fields: std::collections::HashSet<#sort_field> = sort.iter().map(|details| details.field).collect();

                    if sort.len() == fields.len() {
                        Ok(())
                    } else {
                        let mut err = validator::ValidationError::new("sort");
                        err.message = Some("Duplicate sort field".into());
                        return Err(err);
                    }
                }
            }
        }
    }
}
