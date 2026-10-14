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

/// An ORM attribute naming the columns of an entity: its name, the option
/// renaming a field's column, and the options marking a field as no column.
struct OrmAttribute {
    name: &'static str,
    rename: &'static str,
    not_a_column: &'static [&'static str],
}

/// The ORM attributes `Columns` reads: `sqlx`'s and `sea_orm`'s.
const ORM_ATTRIBUTES: [OrmAttribute; 2] = [
    OrmAttribute {
        name: "sqlx",
        rename: "rename",
        not_a_column: &["skip", "flatten"],
    },
    OrmAttribute {
        name: "sea_orm",
        rename: "column_name",
        not_a_column: &["ignore"],
    },
];

/// What the ORM attributes say about an entity field.
#[derive(Default)]
struct OrmField {
    /// `#[sqlx(rename = "...")]` / `#[sea_orm(column_name = "...")]`: the
    /// column name, as written.
    rename: Option<syn::LitStr>,
    /// `#[sqlx(skip)]` / `#[sqlx(flatten)]` / `#[sea_orm(ignore)]`: not a
    /// column of the table.
    is_not_a_column: bool,
}

/// Consume the value of an ORM option this derive does not interpret
/// (`key = value`, `key(...)` or a bare `key`).
fn skip_option(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        let _: syn::Expr = meta.value()?.parse()?;
    } else if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        let _: proc_macro2::TokenStream = content.parse()?;
    }

    Ok(())
}

/// Read the ORM attributes of an entity field, consuming their other options
/// without interpreting them. Two renames of the field must agree.
fn orm_field(field: &syn::Field) -> syn::Result<OrmField> {
    let mut orm_field = OrmField::default();

    for orm in &ORM_ATTRIBUTES {
        for attribute in field
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident(orm.name))
        {
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident(orm.rename) {
                    let column: syn::LitStr = meta.value()?.parse()?;
                    if let Some(existing) = &orm_field.rename
                        && existing.value() != column.value()
                    {
                        let message = format!(
                            "The column is named both `{}` and `{}`",
                            existing.value(),
                            column.value()
                        );
                        let help = "Give the column a single name";
                        return Err(syn::Error::new(
                            column.span(),
                            crate::errors::message(&message, help),
                        ));
                    }
                    orm_field.rename = Some(column);
                } else if orm.not_a_column.iter().any(|key| meta.path.is_ident(key)) {
                    orm_field.is_not_a_column = true;
                } else {
                    skip_option(&meta)?;
                }

                Ok(())
            })?;
        }
    }

    Ok(orm_field)
}

/// Reject `rename_all` in an ORM attribute of the entity
/// (`#[sqlx(rename_all = "...")]`, `#[sea_orm(rename_all = "...")]`): it
/// renames every column in a way this derive does not model, so a mapping
/// taken from the field names would be silently wrong.
fn reject_rename_all(attributes: &[syn::Attribute]) -> syn::Result<()> {
    for orm in &ORM_ATTRIBUTES {
        for attribute in attributes
            .iter()
            .filter(|attribute| attribute.path().is_ident(orm.name))
        {
            let mut rename_all = None;
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename_all") {
                    rename_all = Some(meta.path.clone());
                }
                skip_option(&meta)
            })?;

            if let Some(path) = rename_all {
                let message = format!(
                    "#[derive(Columns)] does not support #[{}(rename_all = \"...\")]",
                    orm.name
                );
                let help = format!(
                    "Name each renamed column with #[{}({} = \"...\")] on its field instead",
                    orm.name, orm.rename
                );
                return crate::errors::abort_spanned(path, &message, &help);
            }
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
/// `rename_all` in its ORM attributes, and of malformed or disagreeing field
/// attributes.
pub fn parse(input: &Input) -> syn::Result<Parsed<'_>> {
    let syn::Data::Struct(data_struct) = &input.data else {
        let message = "Cannot call #[derive(Columns)] on any other item, other than a struct";
        let help = "Call #[derive(Columns)] on the entity struct of a table";
        return crate::errors::abort_call_site(message, help);
    };

    let syn::Fields::Named(named_fields) = &data_struct.fields else {
        let message = "Cannot call #[derive(Columns)] on a struct that doesn't have named fields";
        let help = "Call #[derive(Columns)] on the entity struct of a table";
        return crate::errors::abort_spanned(&input.ident, message, help);
    };

    reject_rename_all(&input.attributes)?;

    let mut columns = Vec::<Column>::new();
    let mut errors = crate::errors::Errors::default();

    for field in &named_fields.named {
        let Some(field_ident) = &field.ident else {
            continue;
        };

        let Some(orm_field) = errors.check(orm_field(field)) else {
            continue;
        };
        if orm_field.is_not_a_column {
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
            name: orm_field
                .rename
                .map_or_else(|| field_ident.to_string(), |column| column.value()),
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

    // A SeaORM entity: `#[sea_orm(column_name)]` renames the column, an
    // ignored field is no column, and its other options are left alone.
    #[test]
    fn a_sea_orm_entity_names_its_columns() {
        let columns = columns(quote::quote! {
            #[derive(DeriveEntityModel, Columns)]
            #[sea_orm(table_name = "orders")]
            #[columns(model = OrderModel)]
            struct Model {
                #[sea_orm(primary_key, auto_increment = false)]
                id: i32,
                #[sea_orm(column_name = "order_number", column_type = "Text", unique)]
                number: String,
                #[domain_field(delivery_date)]
                #[sea_orm(column_name = "delivered_on")]
                delivered_at: DateTime<Utc>,
                #[sea_orm(ignore)]
                cached: bool,
            }
        });

        pretty_assertions::assert_eq!(
            columns,
            vec![
                (String::from("id"), false, String::from("id")),
                (String::from("number"), false, String::from("order_number")),
                (
                    String::from("delivery_date"),
                    true,
                    String::from("delivered_on")
                ),
            ]
        );
    }

    // sqlx and SeaORM may both name a column, if they agree.
    #[test]
    fn agreeing_renames_name_the_column() {
        let columns = columns(quote::quote! {
            struct Order {
                #[sqlx(rename = "order_number")]
                #[sea_orm(column_name = "order_number")]
                number: String,
            }
        });

        pretty_assertions::assert_eq!(
            columns,
            vec![(String::from("number"), false, String::from("order_number"))]
        );
    }

    #[rstest]
    // Not a struct with named fields.
    #[case(quote::quote! { enum Order { A } })]
    #[case(quote::quote! { struct Order(i32); })]
    // A column naming model this derive does not follow.
    #[case(quote::quote! { #[sqlx(rename_all = "camelCase")] struct Order { id: i32 } })]
    #[case(quote::quote! { #[sea_orm(rename_all = "camelCase")] struct Order { id: i32 } })]
    // Two names for one column.
    #[case(quote::quote! {
        struct Order {
            #[sqlx(rename = "number")]
            #[sea_orm(column_name = "order_number")]
            number: String,
        }
    })]
    // Two columns storing the same domain field.
    #[case(quote::quote! { struct Order { id: i32, #[domain_field(id)] order_id: i32 } })]
    // A malformed `#[domain_field(...)]`.
    #[case(quote::quote! { struct Order { #[domain_field()] id: i32 } })]
    fn invalid_entities_are_rejected(#[case] macro_input: proc_macro2::TokenStream) {
        assert!(super::parse(&input(macro_input)).is_err());
    }
}
