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
    /// The column's name in Diesel's `table!`: `#[diesel(column_name)]`, or
    /// the entity field's own name.
    pub diesel_ident: syn::Ident,
}

pub struct Parsed<'a> {
    pub ident: &'a syn::Ident,
    pub generics: &'a syn::Generics,
    pub columns: Vec<Column>,
    /// `#[diesel(table_name = path)]`: the entity is a Diesel one, its columns
    /// those of the `table!` at `path`.
    pub diesel_table: Option<syn::Path>,
}

/// An ORM attribute naming the columns of an entity: its name, the option
/// renaming a field's column, and the options marking a field as no column.
struct OrmAttribute {
    name: &'static str,
    rename: &'static str,
    not_a_column: &'static [&'static str],
}

/// The ORM attributes `Columns` reads: `sqlx`'s, `sea_orm`'s and `diesel`'s.
const ORM_ATTRIBUTES: [OrmAttribute; 3] = [
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
    OrmAttribute {
        name: "diesel",
        rename: "column_name",
        not_a_column: &["embed", "select_expression"],
    },
];

/// What the ORM attributes say about an entity field.
#[derive(Default)]
struct OrmField {
    /// `#[sqlx(rename = "...")]` / `#[sea_orm(column_name = "...")]` /
    /// `#[diesel(column_name = ...)]`: the column name, as written.
    rename: Option<syn::LitStr>,
    /// `#[diesel(column_name = ...)]`: the column's name in Diesel's `table!`.
    diesel_column: Option<syn::Ident>,
    /// `#[sqlx(skip)]` / `#[sqlx(flatten)]` / `#[sea_orm(ignore)]` /
    /// `#[diesel(embed)]` / `#[diesel(select_expression = ...)]`: not a column
    /// of the table.
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

/// A column name, written as a string or (Diesel's way) as an identifier.
fn column_name(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<syn::LitStr> {
    let value = meta.value()?;
    if value.peek(syn::LitStr) {
        return value.parse();
    }

    let ident: syn::Ident = value.parse()?;
    Ok(syn::LitStr::new(
        ident.to_string().trim_start_matches("r#"),
        ident.span(),
    ))
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
                    let column = column_name(&meta)?;
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
                    if orm.name == "diesel" {
                        orm_field.diesel_column = Some(column.parse().map_err(|_| {
                            let message = format!(
                                "`{}` is not a Rust identifier, as Diesel's column names are",
                                column.value()
                            );
                            let help = "Name the column as it is declared in `table!`";
                            syn::Error::new(column.span(), crate::errors::message(&message, help))
                        })?);
                    }
                    orm_field.rename = Some(column);
                } else if orm.not_a_column.iter().any(|key| meta.path.is_ident(key)) {
                    orm_field.is_not_a_column = true;
                    skip_option(&meta)?;
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

/// `#[diesel(table_name = path)]` on the entity: the Diesel `table!` its
/// columns are those of, if it is a Diesel entity.
fn diesel_table(attributes: &[syn::Attribute]) -> syn::Result<Option<syn::Path>> {
    let mut table = None;
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("diesel"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("table_name") {
                table = Some(meta.value()?.parse()?);
                return Ok(());
            }
            skip_option(&meta)
        })?;
    }

    Ok(table)
}

/// `#[columns(skip)]` on an entity field: a column storing no domain field,
/// left out of the columns (e.g. a Diesel column of a SQL type queryweave
/// cannot convert, such as a `Uuid` primary key).
fn is_skipped(field: &syn::Field) -> syn::Result<Option<syn::Ident>> {
    use crate::attributes::OptionKind;

    crate::attributes::flag_option(
        &field.attrs,
        "columns",
        "skip",
        &[("skip", OptionKind::Flag)],
    )
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
    let diesel_table = diesel_table(&input.attributes)?;

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
        let Some(skipped) = errors.check(is_skipped(field)) else {
            continue;
        };
        if skipped.is_some() {
            if let Some(mapping) = field
                .attrs
                .iter()
                .find(|attribute| attribute.path().is_ident("domain_field"))
            {
                let message = "A skipped column stores no domain field";
                let help = "Remove either #[columns(skip)] or #[domain_field(...)]";
                errors.push(syn::Error::new_spanned(
                    mapping,
                    crate::errors::message(message, help),
                ));
            }
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
            diesel_ident: orm_field
                .diesel_column
                .unwrap_or_else(|| field_ident.clone()),
            domain_field,
        });
    }

    errors.finish(Parsed {
        ident: &input.ident,
        generics: &input.generics,
        columns,
        diesel_table,
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

    // A Diesel entity: its `table!` from `#[diesel(table_name)]`, a column
    // renamed by `#[diesel(column_name)]` (an identifier), embedded and
    // computed fields no columns, and a `#[columns(skip)]` column left out.
    #[test]
    fn a_diesel_entity_names_its_table_and_columns() {
        let input = input(quote::quote! {
            #[derive(Queryable, Selectable, Columns)]
            #[diesel(table_name = crate::schema::orders, check_for_backend(diesel::pg::Pg))]
            #[columns(model = OrderModel)]
            struct Order {
                #[columns(skip)]
                id: Uuid,
                #[diesel(column_name = order_number)]
                number: String,
                #[domain_field(delivery_date)]
                delivered_at: DateTime<Utc>,
                #[diesel(embed)]
                lines: Lines,
                #[diesel(select_expression = orders::number.concat("!"))]
                shout: String,
            }
        });
        let parsed = super::parse(&input).unwrap();

        pretty_assertions::assert_eq!(
            parsed.diesel_table,
            Some(syn::parse_quote!(crate::schema::orders))
        );
        pretty_assertions::assert_eq!(
            parsed
                .columns
                .iter()
                .map(|column| (
                    column.domain_field.to_string(),
                    column.name.clone(),
                    column.diesel_ident.to_string()
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    String::from("number"),
                    String::from("order_number"),
                    String::from("order_number")
                ),
                (
                    String::from("delivery_date"),
                    String::from("delivered_at"),
                    String::from("delivered_at")
                ),
            ]
        );
    }

    // Without `#[diesel(table_name)]` an entity is not a Diesel one.
    #[test]
    fn an_entity_without_a_diesel_table_is_not_a_diesel_one() {
        let input = input(quote::quote! { struct Order { id: i32 } });

        assert!(super::parse(&input).unwrap().diesel_table.is_none());
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
    // A skipped column naming the domain field it stores.
    #[case(quote::quote! { struct Order { #[columns(skip)] #[domain_field(id)] order_id: i32 } })]
    // An unknown or valued `#[columns(...)]` field option.
    #[case(quote::quote! { struct Order { #[columns(hide)] id: i32 } })]
    #[case(quote::quote! { struct Order { #[columns(skip = true)] id: i32 } })]
    // A Diesel column name that is no identifier.
    #[case(quote::quote! { struct Order { #[diesel(column_name = "order number")] number: String } })]
    fn invalid_entities_are_rejected(#[case] macro_input: proc_macro2::TokenStream) {
        assert!(super::parse(&input(macro_input)).is_err());
    }
}
