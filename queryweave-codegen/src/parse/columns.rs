/// Struct for encapsulating the proc macro derive input for an entity.
#[derive(derive_new::new)]
pub struct Input {
    ident: syn::Ident,
    generics: syn::Generics,
    data: syn::Data,
    pub attributes: Vec<syn::Attribute>,
}

/// One column of the entity and the domain field it stores.
#[derive(Debug, PartialEq, Eq)]
pub struct Column {
    /// The domain field stored, carrying the span of what the user wrote.
    pub domain_field: syn::Ident,
    /// Whether the domain field was named by `#[domain_field(name)]` (and so
    /// must be checked against the domain model) rather than taken from the
    /// entity field's own name.
    pub is_mapped: bool,
    /// The column name.
    pub name: String,
}

pub struct Parsed<'a> {
    pub ident: &'a syn::Ident,
    pub generics: &'a syn::Generics,
    pub columns: Vec<Column>,
}

/// What `#[sqlx(...)]` says about an entity field.
#[derive(Default)]
struct SqlxField {
    /// `#[sqlx(rename = "...")]`: the column name.
    rename: Option<String>,
    /// `#[sqlx(skip)]` / `#[sqlx(flatten)]`: not a column of the table.
    is_not_a_column: bool,
}

/// Read the `#[sqlx(...)]` attributes of an entity field, consuming any other
/// `sqlx` options without interpreting them.
fn sqlx_field(field: &syn::Field) -> syn::Result<SqlxField> {
    let mut sqlx_field = SqlxField::default();

    for attribute in field
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("sqlx"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let column: syn::LitStr = meta.value()?.parse()?;
                sqlx_field.rename = Some(column.value());
            } else if meta.path.is_ident("skip") || meta.path.is_ident("flatten") {
                sqlx_field.is_not_a_column = true;
            } else if meta.input.peek(syn::Token![=]) {
                let _: syn::Expr = meta.value()?.parse()?;
            } else if meta.input.peek(syn::token::Paren) {
                let content;
                syn::parenthesized!(content in meta.input);
                let _: proc_macro2::TokenStream = content.parse()?;
            }

            Ok(())
        })?;
    }

    Ok(sqlx_field)
}

/// Reject `#[sqlx(rename_all = "...")]` on the entity: it renames every
/// column in a way this derive does not model, so a mapping taken from the
/// field names would be silently wrong.
fn reject_rename_all(attributes: &[syn::Attribute]) -> syn::Result<()> {
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("sqlx"))
    {
        let mut rename_all = None;
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                rename_all = Some(meta.path.clone());
            }
            if meta.input.peek(syn::Token![=]) {
                let _: syn::Expr = meta.value()?.parse()?;
            }

            Ok(())
        })?;

        if let Some(path) = rename_all {
            let message = "#[derive(Columns)] does not support #[sqlx(rename_all = \"...\")]";
            let help =
                "Name each renamed column with #[sqlx(rename = \"...\")] on its field instead";
            return crate::errors::abort_spanned(path, message, help);
        }
    }

    Ok(())
}

/// Parse the input of a `Columns` derive: its columns and the domain fields
/// they store.
///
/// # Errors
///
/// Returns the errors of an input that is not a struct with named fields, of
/// `#[sqlx(rename_all)]`, and of malformed field attributes.
pub fn parse(input: &Input) -> syn::Result<Parsed<'_>> {
    let syn::Data::Struct(data_struct) = &input.data else {
        let message = "Cannot call #[derive(Columns)] on any other item, other than a struct";
        let help = "Call #[derive(Columns)] on the sqlx entity struct of a table";
        return crate::errors::abort_call_site(message, help);
    };

    let syn::Fields::Named(named_fields) = &data_struct.fields else {
        let message = "Cannot call #[derive(Columns)] on a struct that doesn't have named fields";
        let help = "Call #[derive(Columns)] on the sqlx entity struct of a table";
        return crate::errors::abort_spanned(&input.ident, message, help);
    };

    reject_rename_all(&input.attributes)?;

    let mut columns = Vec::<Column>::new();
    let mut errors = crate::errors::Errors::default();

    for field in &named_fields.named {
        let Some(field_ident) = &field.ident else {
            continue;
        };

        let Some(sqlx_field) = errors.check(sqlx_field(field)) else {
            continue;
        };
        if sqlx_field.is_not_a_column {
            continue;
        }

        let Some(domain_field) =
            errors.check(crate::attributes::mapped_field_name(field, "domain_field"))
        else {
            continue;
        };

        if let Some(existing) = columns
            .iter()
            .find(|column| column.domain_field == domain_field)
        {
            let message = format!(
                "The domain field `{domain_field}` is already stored in the column `{}`",
                existing.name
            );
            let help = "Map each domain field onto a single column";
            errors.push(syn::Error::new(
                domain_field.span(),
                crate::errors::message(&message, help),
            ));
            continue;
        }

        columns.push(Column {
            is_mapped: domain_field != *field_ident,
            name: sqlx_field.rename.unwrap_or_else(|| field_ident.to_string()),
            domain_field,
        });
    }

    errors.finish(Parsed {
        ident: &input.ident,
        generics: &input.generics,
        columns,
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    fn input(macro_input: proc_macro2::TokenStream) -> super::Input {
        let syn::DeriveInput {
            ident,
            generics,
            data,
            attrs,
            ..
        } = syn::parse2(macro_input).unwrap();

        super::Input::new(ident, generics, data, attrs)
    }

    fn columns(macro_input: proc_macro2::TokenStream) -> Vec<(String, bool, String)> {
        let input = input(macro_input);
        let parsed = super::parse(&input).unwrap();

        parsed
            .columns
            .into_iter()
            .map(|column| {
                (
                    column.domain_field.to_string(),
                    column.is_mapped,
                    column.name,
                )
            })
            .collect()
    }

    // Each field is a column storing the domain field of its name, unless
    // `#[domain_field]` or `#[sqlx(rename)]` say otherwise; other `sqlx`
    // options are left alone, and skipped / flattened fields are not columns.
    #[test]
    fn each_field_is_a_column_storing_a_domain_field() {
        let columns = columns(quote::quote! {
            #[derive(sqlx::FromRow, Columns)]
            #[columns(model = OrderModel)]
            struct Order {
                id: i32,
                #[domain_field(delivery_date)]
                delivered_at: DateTime<Utc>,
                #[sqlx(rename = "order_number")]
                number: String,
                #[sqlx(default)]
                client: Option<String>,
                #[sqlx(try_from = "i64")]
                capacity: i32,
                #[sqlx(skip)]
                cached: bool,
                #[sqlx(flatten)]
                lines: Lines,
            }
        });

        pretty_assertions::assert_eq!(
            columns,
            vec![
                (String::from("id"), false, String::from("id")),
                (
                    String::from("delivery_date"),
                    true,
                    String::from("delivered_at")
                ),
                (String::from("number"), false, String::from("order_number")),
                (String::from("client"), false, String::from("client")),
                (String::from("capacity"), false, String::from("capacity")),
            ]
        );
    }

    #[rstest]
    // Not a struct with named fields.
    #[case(quote::quote! { enum Order { A } })]
    #[case(quote::quote! { struct Order(i32); })]
    // A column naming model this derive does not follow.
    #[case(quote::quote! { #[sqlx(rename_all = "camelCase")] struct Order { id: i32 } })]
    // Two columns storing the same domain field.
    #[case(quote::quote! { struct Order { id: i32, #[domain_field(id)] order_id: i32 } })]
    // A malformed `#[domain_field(...)]`.
    #[case(quote::quote! { struct Order { #[domain_field()] id: i32 } })]
    fn invalid_entities_are_rejected(#[case] macro_input: proc_macro2::TokenStream) {
        assert!(super::parse(&input(macro_input)).is_err());
    }
}
