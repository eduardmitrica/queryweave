use syn::spanned::Spanned;

/// The SQL comparison a single generated filter field performs.
///
/// Decided here, from the `#[filter(...)]` meta, so the generated code can pass
/// it on as-is instead of re-deriving it from the filter field's name at runtime.
/// Which types an operator applies to is not checked here: the generated code
/// asserts it through `queryweave`'s filter-kind traits, so the compiler
/// decides from the actual type rather than the macro guessing from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOperator {
    /// `lt`: `column < value`.
    LessThan,
    /// `eq`: `column = value`.
    Equal,
    /// `gt`: `column > value`.
    GreaterThan,
    /// `sw`: `column LIKE 'value%'`.
    StartsWith,
    /// `c`: `column LIKE '%value%'`.
    Contains,
    /// `ew`: `column LIKE '%value'`.
    EndsWith,
    /// `bool`: `column = value`, on a boolean.
    Boolean,
}

impl FilterOperator {
    /// Map a `#[filter(...)]` meta ident to its operator.
    #[must_use]
    pub fn from_meta_ident(ident: &syn::Ident) -> Option<Self> {
        match ident.to_string().as_str() {
            "lt" => Some(Self::LessThan),
            "eq" => Some(Self::Equal),
            "gt" => Some(Self::GreaterThan),
            "sw" => Some(Self::StartsWith),
            "c" => Some(Self::Contains),
            "ew" => Some(Self::EndsWith),
            "bool" => Some(Self::Boolean),
            _ => None,
        }
    }

    /// The meta the operator is written as in `#[filter(...)]`; what it adds
    /// to a filter field's name is up to the conventions
    /// (`Conventions::filter_field`).
    #[must_use]
    pub const fn meta(self) -> &'static str {
        match self {
            Self::LessThan => "lt",
            Self::Equal => "eq",
            Self::GreaterThan => "gt",
            Self::StartsWith => "sw",
            Self::Contains => "c",
            Self::EndsWith => "ew",
            Self::Boolean => "bool",
        }
    }

    /// How the operator compares a field with the given value, for the
    /// generated filter fields' docs (e.g. "starts with").
    #[must_use]
    pub const fn phrase(self) -> &'static str {
        match self {
            Self::LessThan => "is less than",
            Self::Equal | Self::Boolean => "equals",
            Self::GreaterThan => "is greater than",
            Self::StartsWith => "starts with",
            Self::Contains => "contains",
            Self::EndsWith => "ends with",
        }
    }
}

/// The result of interpreting the parser's output.
#[derive(Debug, Default, PartialEq, Eq, derive_new::new)]
pub struct Model {
    #[new(default)]
    pub field_idents: Vec<syn::Ident>,
    /// The ident each filter field (same index in `field_idents`) has on the
    /// equivalent type (see `#[equivalent_field(name)]`), carrying the span of
    /// what the user wrote.
    #[new(default)]
    pub equivalent_field_idents: Vec<syn::Ident>,
    /// The value type each filter field (same index in `field_idents`) is
    /// derived from: the struct field's type, less one outer `Option`.
    #[new(default)]
    pub field_value_types: Vec<syn::Type>,
    /// The column each filter field (same index in `field_idents`) compares against.
    #[new(default)]
    pub field_columns: Vec<String>,
    /// The operator each filter field (same index in `field_idents`) compares with.
    #[new(default)]
    pub field_operators: Vec<FilterOperator>,
    #[new(default)]
    pub sort_fields: Vec<syn::Ident>,
    #[new(default)]
    pub snake_cased_sort_fields: Vec<syn::Ident>,
    /// The sort variant each sort field (same index in `sort_fields`) has on
    /// the equivalent type, named by the conventions after its equivalent
    /// field name.
    #[new(default)]
    pub equivalent_sort_fields: Vec<syn::Ident>,
}

/// A field's own ident and the one it has on the equivalent type.
struct FieldNames<'a> {
    ident: &'a syn::Ident,
    equivalent: &'a syn::Ident,
}

impl Model {
    fn push_field_details<C: crate::conventions::Conventions>(
        &mut self,
        operator: FilterOperator,
        field: &syn::Field,
        names: &FieldNames<'_>,
        nested: &syn::meta::ParseNestedMeta<'_>,
    ) -> Result<(), syn::Error> {
        let field_ident = names.ident;
        let new_field_ident =
            C::filter_field(field_ident, operator.meta(), proc_macro2::Span::call_site());

        if self.field_idents.contains(&new_field_ident) {
            return Err(nested.error(format!(
                "Duplicate filter on the same field ({field_ident}): {}",
                operator.meta()
            )));
        }

        // Whether the field is optional is a matter of shape, not of type: the
        // generated filter is optional either way.
        let value_type = crate::utils::unwrap_option(&field.ty).unwrap_or(&field.ty);

        self.field_idents.push(new_field_ident);
        // Spanned to what the user wrote, so a name missing on the equivalent
        // type is reported there.
        // The same conventions name the equivalent type's filter fields: the
        // layers must share them for member names (see `Conventions`).
        self.equivalent_field_idents.push(C::filter_field(
            names.equivalent,
            operator.meta(),
            names.equivalent.span(),
        ));
        self.field_value_types.push(value_type.clone());
        self.field_columns.push(field_ident.to_string());
        self.field_operators.push(operator);

        Ok(())
    }

    fn push_sort_details<C: crate::conventions::Conventions>(
        &mut self,
        names: &FieldNames<'_>,
        inner: &syn::Path,
    ) -> Result<(), syn::Error> {
        let field_ident = names.ident;
        let sort_variant = C::sort_variant(field_ident, proc_macro2::Span::call_site());

        if self.sort_fields.contains(&sort_variant) {
            let message =
                format!("Duplicate #[sortable] attribute on the same field ({field_ident})");
            let help = format!("Annotate the field ({field_ident}) only once with #[sortable]");
            return crate::errors::abort(inner.span(), message.as_str(), help.as_str());
        }

        self.sort_fields.push(sort_variant);
        self.snake_cased_sort_fields.push(field_ident.clone());
        self.equivalent_sort_fields
            .push(C::sort_variant(names.equivalent, names.equivalent.span()));

        Ok(())
    }
}

/// Analyze the fields of a `Filters` derive (domain or API) into its filters
/// and sort fields, named by the conventions `C`.
///
/// # Errors
///
/// Returns the errors of malformed `#[filter(...)]`, `#[sortable]` and
/// `#[equivalent_field(...)]` attributes, and of a struct with neither.
pub fn analyze<C: crate::conventions::Conventions>(
    parsed: &crate::parse::filters::Parsed<'_>,
) -> syn::Result<Model> {
    let mut model = Model::new();
    let mut errors = crate::errors::Errors::default();

    for field in &parsed.named_fields.named {
        let Some(field_ident) = &field.ident else {
            continue;
        };

        // The name the field has on the equivalent type (only the API
        // derive maps onto one; for the domain derive it is the field's own).
        let Some(equivalent) = errors.check(crate::attributes::equivalent_field_name(field)) else {
            continue;
        };
        let names = FieldNames {
            ident: field_ident,
            equivalent: &equivalent,
        };

        // Each #[filter(...)] or #[sortable] meta for the field
        for attr in &field.attrs {
            let result = if attr.path().is_ident("filter") {
                ParseFiltered::parse::<C>(attr, &mut model, field, &names)
            } else if attr.path().is_ident("sortable") {
                ParseSortable::parse::<C>(attr, &mut model, &names)
            } else {
                continue;
            };

            errors.check(result);
        }
    }

    let model = errors.finish(model)?;

    if model.field_idents.is_empty() && model.sort_fields.is_empty() {
        let message =
            "Cannot call #[derive(Filters)] on a struct with no #[filter(...)] attributes";
        let help = "Place at least one #[filter(...)] on a named field of the struct";
        return crate::errors::abort_call_site(message, help);
    }

    Ok(model)
}

trait ParseFiltered {
    fn parse<C: crate::conventions::Conventions>(
        &self,
        model: &mut Model,
        field: &syn::Field,
        names: &FieldNames<'_>,
    ) -> Result<(), syn::Error>;
}

impl ParseFiltered for syn::Attribute {
    fn parse<C: crate::conventions::Conventions>(
        &self,
        model: &mut Model,
        field: &syn::Field,
        names: &FieldNames<'_>,
    ) -> Result<(), syn::Error> {
        let list = self.meta.require_list()?;
        if list.tokens.is_empty() {
            let message =
                "The list of arguments of the #[filter(...)] attribute cannot be left empty";
            let help = "Provide an argument to the #[filter(...)] attribute";
            return crate::errors::abort_spanned(list, message, help);
        }

        self.parse_nested_meta(|meta| {
            let Some(operator) = meta.path.get_ident().and_then(FilterOperator::from_meta_ident)
            else {
                let message =
                    "Unrecognized filter meta; expected one of: lt, eq, gt, sw, c, ew, bool";
                let help = "Place inside the #[filter(...)] attribute one of the following meta: lt, eq, gt, sw, c, ew, bool";
                return Err(meta.error(crate::errors::message(message, help)));
            };

            model.push_field_details::<C>(operator, field, names, &meta)
        })
    }
}

trait ParseSortable {
    fn parse<C: crate::conventions::Conventions>(
        &self,
        model: &mut Model,
        names: &FieldNames<'_>,
    ) -> Result<(), syn::Error>;
}

impl ParseSortable for syn::Attribute {
    fn parse<C: crate::conventions::Conventions>(
        &self,
        model: &mut Model,
        names: &FieldNames<'_>,
    ) -> Result<(), syn::Error> {
        match &self.meta {
            syn::Meta::Path(inner) => model.push_sort_details::<C>(names, inner),
            syn::Meta::List(list) => {
                let message = "Cannot place #[sortable(...)] with inner attributes on a field";
                let help = "Annotate the field with #[sortable]";
                crate::errors::abort(list.span(), message, help)
            },
            syn::Meta::NameValue(name_value) => {
                let message = "Cannot place #[sortable = ...] as name = value on a field";
                let help = "Annotate the field with #[sortable]";
                crate::errors::abort(name_value.span(), message, help)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FilterOperator;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    fn idents() -> crate::conventions::FiltersNames {
        crate::conventions::FiltersNames {
            query: quote::format_ident!("UsersQueryModel"),
            filters: quote::format_ident!("UsersFilterOptionsModel"),
            sort_field: quote::format_ident!("UserSortFieldModel"),
            sort_details: quote::format_ident!("UserSortDetailsModel"),
            input: quote::format_ident!("UserModel"),
            page: quote::format_ident!("UsersPageModel"),
            query_params: quote::format_ident!("UsersQueryParams"),
        }
    }

    fn analyze_named_fields(
        named_fields_input: proc_macro2::TokenStream,
    ) -> syn::Result<super::Model> {
        let named_fields = &syn::parse2(named_fields_input).unwrap();
        let parsed = crate::parse::filters::Parsed {
            idents: idents(),
            named_fields,
        };

        super::analyze::<crate::conventions::DefaultConventions>(&parsed)
    }

    fn ty(source: &str) -> syn::Type {
        syn::parse_str(source).unwrap()
    }

    // Every filter operator must be recognized on its own and must produce the
    // `<field>_<op>` filter ident together with the operator it stands for.
    #[rstest]
    #[case(quote::quote! { { #[filter(sw)] username: String, } }, "username_sw", FilterOperator::StartsWith)]
    #[case(quote::quote! { { #[filter(c)] username: String, } }, "username_c", FilterOperator::Contains)]
    #[case(quote::quote! { { #[filter(ew)] username: String, } }, "username_ew", FilterOperator::EndsWith)]
    #[case(quote::quote! { { #[filter(eq)] username: String, } }, "username_eq", FilterOperator::Equal)]
    #[case(quote::quote! { { #[filter(lt)] value: f64, } }, "value_lt", FilterOperator::LessThan)]
    #[case(quote::quote! { { #[filter(gt)] value: f64, } }, "value_gt", FilterOperator::GreaterThan)]
    // `bool` is the odd one out: no suffix, so the filter ident is the bare
    // field name.
    #[case(quote::quote! { { #[filter(bool)] is_active: bool, } }, "is_active", FilterOperator::Boolean)]
    fn test_each_filter_operator_is_recognized(
        #[case] named_fields_input: proc_macro2::TokenStream,
        #[case] expected_field_ident: &str,
        #[case] expected_operator: FilterOperator,
    ) {
        let model = analyze_named_fields(named_fields_input).unwrap();

        assert_eq!(
            model.field_idents,
            vec![quote::format_ident!("{}", expected_field_ident)]
        );
        assert_eq!(model.field_operators, vec![expected_operator]);
    }

    // The value type is the field type less one outer `Option`, whatever the
    // type is: a value object needs no generic argument, and a type the macro
    // could not have recognized by name (an alias, `NaiveDate`, ...) is passed
    // through for the compiler to resolve.
    #[rstest]
    #[case(quote::quote! { { #[filter(eq)] number: Option<OrderNumber<String>>, } }, "OrderNumber<String>")]
    #[case(quote::quote! { { #[filter(eq)] number: OrderNumber, } }, "OrderNumber")]
    #[case(quote::quote! { { #[filter(lt)] price: Price, } }, "Price")]
    #[case(quote::quote! { { #[filter(lt)] due: Option<chrono::NaiveDate>, } }, "chrono::NaiveDate")]
    fn test_the_value_type_is_the_field_type_less_an_option(
        #[case] named_fields_input: proc_macro2::TokenStream,
        #[case] expected_value_type: &str,
    ) {
        let model = analyze_named_fields(named_fields_input).unwrap();

        assert_eq!(model.field_value_types, vec![ty(expected_value_type)]);
    }

    // Whether an operator fits a type is for the compiler to decide (through
    // the generated filter-kind assertions), so the analysis accepts it.
    #[test]
    fn test_operator_and_type_compatibility_is_left_to_the_compiler() {
        assert!(
            analyze_named_fields(quote::quote! { { #[filter(lt)] username: String, } }).is_ok()
        );
    }

    // The column is the field name verbatim. The filter suffix is only part of
    // the filter field ident, so a field whose own name contains something that
    // looks like a suffix (`_c`, `_sw`, `_eq`, ...) keeps it intact.
    #[rstest]
    #[case(quote::quote! { { #[filter(bool)] is_closed: bool, } }, "is_closed")]
    #[case(quote::quote! { { #[filter(eq)] user_count: i32, } }, "user_count")]
    #[case(quote::quote! { { #[filter(sw)] is_sweet_name: String, } }, "is_sweet_name")]
    #[case(quote::quote! { { #[filter(lt)] is_equal_to: i32, } }, "is_equal_to")]
    #[case(quote::quote! { { #[filter(c)] name_c: String, } }, "name_c")]
    fn test_the_column_is_the_unmodified_field_name(
        #[case] named_fields_input: proc_macro2::TokenStream,
        #[case] expected_column: &str,
    ) {
        let model = analyze_named_fields(named_fields_input).unwrap();

        assert_eq!(model.field_columns, vec![String::from(expected_column)]);
    }

    // Several operators on one field share its column and value type, each
    // keeping its own operator, index-aligned with `field_idents`.
    #[test]
    fn test_filter_details_are_aligned_with_the_filter_fields() {
        let model = analyze_named_fields(quote::quote! {
            {
                #[filter(lt, gt)]
                value: f64,
                #[filter(bool)]
                is_active: bool,
                not_filtered: PasswordHash,
            }
        })
        .unwrap();

        let expected_model = super::Model {
            field_idents: vec![
                quote::format_ident!("value_lt"),
                quote::format_ident!("value_gt"),
                quote::format_ident!("is_active"),
            ],
            equivalent_field_idents: vec![
                quote::format_ident!("value_lt"),
                quote::format_ident!("value_gt"),
                quote::format_ident!("is_active"),
            ],
            field_value_types: vec![ty("f64"), ty("f64"), ty("bool")],
            field_columns: vec![
                String::from("value"),
                String::from("value"),
                String::from("is_active"),
            ],
            field_operators: vec![
                FilterOperator::LessThan,
                FilterOperator::GreaterThan,
                FilterOperator::Boolean,
            ],
            sort_fields: Vec::new(),
            snake_cased_sort_fields: Vec::new(),
            equivalent_sort_fields: Vec::new(),
        };
        assert_eq!(model, expected_model);
    }

    // A field renamed with `#[equivalent_field(name)]` keeps its own filter
    // and sort names (what the DTO exposes) and gets the equivalent ones
    // (what it maps onto), derived the same way from the equivalent name.
    #[test]
    fn test_a_renamed_field_gets_equivalent_filter_and_sort_names() {
        let model = analyze_named_fields(quote::quote! {
            {
                #[filter(eq, sw)]
                #[sortable]
                #[equivalent_field(number)]
                order_number: String,
            }
        })
        .unwrap();

        assert_eq!(
            model.field_idents,
            vec![
                quote::format_ident!("order_number_eq"),
                quote::format_ident!("order_number_sw"),
            ]
        );
        assert_eq!(
            model.equivalent_field_idents,
            vec![
                quote::format_ident!("number_eq"),
                quote::format_ident!("number_sw"),
            ]
        );
        assert_eq!(model.sort_fields, vec![quote::format_ident!("OrderNumber")]);
        assert_eq!(
            model.equivalent_sort_fields,
            vec![quote::format_ident!("Number")]
        );
    }

    // Malformed attributes are rejected.
    #[rstest]
    // A malformed `#[equivalent_field(...)]`.
    #[case(quote::quote! { { #[filter(sw)] #[equivalent_field()] username: String, } })]
    // An operator that does not exist.
    #[case(quote::quote! { { #[filter(nonsense)] username: String, } })]
    // An empty operator list.
    #[case(quote::quote! { { #[filter()] username: String, } })]
    // Not a list at all.
    #[case(quote::quote! { { #[filter] username: String, } })]
    // The same operator twice on one field.
    #[case(quote::quote! { { #[filter(sw, sw)] username: String, } })]
    // `#[sortable]` must be a bare path.
    #[case(quote::quote! { { #[filter(sw)] #[sortable(extra)] username: String, } })]
    #[case(quote::quote! { { #[filter(sw)] #[sortable = "yes"] username: String, } })]
    // The same field marked `#[sortable]` twice.
    #[case(quote::quote! { { #[filter(sw)] #[sortable] #[sortable] username: String, } })]
    fn test_malformed_attributes_are_rejected(
        #[case] named_fields_input: proc_macro2::TokenStream,
    ) {
        assert!(analyze_named_fields(named_fields_input).is_err());
    }

    // A repeated operator is named in the error as it was written.
    #[rstest]
    #[case(quote::quote! { { #[filter(sw, sw)] username: String, } }, ": sw")]
    #[case(quote::quote! { { #[filter(bool, bool)] is_active: bool, } }, ": bool")]
    fn test_a_repeated_filter_is_named_in_the_error(
        #[case] named_fields_input: proc_macro2::TokenStream,
        #[case] expected_operator: &str,
    ) {
        let error = analyze_named_fields(named_fields_input).unwrap_err();

        assert!(
            error.to_string().ends_with(expected_operator),
            "Unexpected error: {error}"
        );
    }

    // Sort fields are pascal cased from the snake cased field name.
    #[test]
    fn test_sort_fields_are_pascal_cased_from_the_field_name() {
        let model = analyze_named_fields(quote::quote! {
            {
                #[filter(lt)]
                #[sortable]
                delivery_date: chrono::DateTime<chrono::Utc>,
            }
        })
        .unwrap();

        assert_eq!(
            model.sort_fields,
            vec![quote::format_ident!("DeliveryDate")]
        );
        assert_eq!(
            model.snake_cased_sort_fields,
            vec![quote::format_ident!("delivery_date")]
        );
    }

    // A struct carrying only #[sortable] (no #[filter(...)]) is legal: it can be
    // sorted but not filtered. Together with the filter-only case below this
    // pins the `&&` in the "no attributes at all" guard: with `||` either
    // half-populated struct would be rejected.
    #[test]
    fn test_analysis_succeeds_with_sortable_but_no_filter() {
        let model = analyze_named_fields(quote::quote! {
            {
                #[sortable]
                username: String,
            }
        })
        .unwrap();

        assert_eq!(model.field_idents, Vec::<syn::Ident>::new());
        assert_eq!(model.sort_fields, vec![quote::format_ident!("Username")]);
    }

    // The mirror image: only #[filter(...)], nothing sortable.
    #[test]
    fn test_analysis_succeeds_with_filter_but_no_sortable() {
        let model = analyze_named_fields(quote::quote! {
            {
                #[filter(sw)]
                username: String,
            }
        })
        .unwrap();

        assert_eq!(
            model.field_idents,
            vec![quote::format_ident!("username_sw")]
        );
        assert_eq!(model.sort_fields, Vec::<syn::Ident>::new());
    }

    // With neither attribute there is nothing to generate, so analysis must
    // fail.
    #[test]
    fn test_analysis_fails_without_filter_or_sortable_attributes() {
        assert!(analyze_named_fields(quote::quote! { { username: String, } }).is_err());
    }
}
