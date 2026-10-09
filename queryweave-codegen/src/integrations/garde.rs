//! `garde`: validation of the query parameters (`page_size` at least 1,
//! `sort` at most one detail per sortable field, none repeated), an
//! alternative to `validator` needing no serializer.

use quote::quote;

/// The `garde` integration.
pub struct Garde;

impl super::Integration for Garde {
    fn name(&self) -> &'static str {
        "garde"
    }

    fn supports_validate_non_nullable(&self) -> bool {
        true
    }

    /// `validate_non_nullable() -> Result<(), garde::Report>`: the struct's
    /// `garde::Validate`, then an error at each field set to `null` that may
    /// not be.
    fn validate_non_nullable(
        &self,
        context: &super::NonNullableContext<'_>,
    ) -> proc_macro2::TokenStream {
        let super::NonNullableContext { ident, flag, .. } = *context;
        // Spanned to the flag: the DTO must derive `garde::Validate`.
        let validate_check = quote::quote_spanned! {flag.span()=>
            // Never called: an anonymous const, so the check adds no name to
            // the user's type.
            const _: fn() = || {
                fn garde_validate<T: garde::Validate>() {}
                garde_validate::<#ident>();
            };
        };

        quote! {
            #validate_check

            impl #ident {
                /// Validate the struct (`garde::Validate`), then reject each
                /// field set to `null` that may not be (see `rejected_nulls`).
                ///
                /// # Errors
                ///
                /// Returns the report, with an error at each field rejected.
                pub fn validate_non_nullable(&self) -> Result<(), garde::Report> {
                    let mut report = match garde::Validate::validate(self) {
                        Ok(()) => garde::Report::new(),
                        Err(report) => report,
                    };
                    for field in self.rejected_nulls() {
                        report.append(garde::Path::new(field), garde::Error::new("cannot be set to null"));
                    }

                    if report.is_empty() {
                        Ok(())
                    } else {
                        Err(report)
                    }
                }
            }
        }
    }

    fn query_params(&self, _context: &super::Context<'_>) -> super::TypeAdditions {
        super::TypeAdditions {
            derives: vec![quote!(garde::Validate)],
            // The filters are not validated: only paging and sorting are.
            attributes: quote!(#[garde(allow_unvalidated)]),
        }
    }

    fn page_size(&self, _context: &super::Context<'_>) -> proc_macro2::TokenStream {
        quote!(#[garde(range(min = 1))])
    }

    fn sort(&self, context: &super::Context<'_>) -> proc_macro2::TokenStream {
        let crate::conventions::FiltersNames {
            sort_field,
            sort_details,
            ..
        } = context.names;

        quote! {
            #[garde(
                length(min = 1, max = #sort_field::COUNT as usize),
                custom(#sort_details::garde_unique_sort)
            )]
        }
    }

    /// The custom rule the sortable query params run on `sort`: an associated
    /// function of the sort details type, so several sortable mirrors can
    /// live in one module.
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
                /// Returns a validation error when a field is repeated.
                pub fn garde_unique_sort(sort: &Option<Vec<Self>>, _context: &()) -> garde::Result {
                    let Some(sort) = sort else {
                        return Ok(());
                    };
                    let fields: std::collections::HashSet<#sort_field> = sort.iter().map(|details| details.field).collect();

                    if sort.len() == fields.len() {
                        Ok(())
                    } else {
                        Err(garde::Error::new("Duplicate sort field"))
                    }
                }
            }
        }
    }
}
