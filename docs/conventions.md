# Your own conventions

queryweave names what it generates by a set of rules, its **conventions**.
The default ones are this repository's:

- `Filters` names a struct's types after its name, split into words, the last
  word being the layer: `UserModel` gets `UsersQueryModel`,
  `UserSortFieldModel`, and so on; `UserDto` gets `UsersQueryParams`,
  `UserSortFieldDto`. A struct's name therefore needs at least two words.
- A filter is the field name followed by the operator (`username_sw`), except
  `bool`, which keeps the field name (`active`).
- A sort variant is the field name in PascalCase (`full_name` gives
  `FullName`).
- A merge patch struct's name starts with `MergePatch`.
- The runtime crates are `::queryweave`, `::queryweave_api` and
  `::queryweave_sql`.
- The integrations are the built-in ones (see
  [integrations.md](integrations.md)).

If your project names things differently, or needs integrations of its own,
you replace these rules with yours.

## Why a crate of your own

A derive macro has to live in a crate of type `proc-macro`, and its rules
are fixed when that crate is compiled. So instead of configuring
`queryweave-derive`, you write a small proc-macro crate that calls
queryweave's code generation, `queryweave-codegen`, with your conventions.
Each of its derives is one line.

## Setting it up

`Cargo.toml`:

```toml
[package]
name = "my-derive"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
queryweave-codegen = "0.2"
proc-macro2 = "1"
quote = "1"
syn = { version = "2", features = ["full"] }
```

`src/lib.rs`:

```rust,ignore
use proc_macro::TokenStream;
use queryweave_codegen::conventions::{Conventions, DefaultConventions, FiltersNames};

struct MyConventions;

impl Conventions for MyConventions {
    // `Customer` → `CustomerSearch`, `CustomerSortKey`, ...: any name works.
    fn filters_names(input: &syn::Ident) -> syn::Result<FiltersNames> {
        Ok(FiltersNames {
            query: quote::format_ident!("{input}Search"),
            filters: quote::format_ident!("{input}Filters"),
            sort_field: quote::format_ident!("{input}SortKey"),
            sort_details: quote::format_ident!("{input}Sort"),
            input: input.clone(),
            page: quote::format_ident!("{input}Page"),
            query_params: quote::format_ident!("{input}SearchParams"),
        })
    }

    // Keep the default rules you do not change.
    fn check_model_name(name: &syn::Ident) -> syn::Result<()> {
        DefaultConventions::check_model_name(name)
    }
    // ... and the other required functions, likewise.
}

#[proc_macro_derive(Filters, attributes(queryweave, filter, sortable, filters, equivalent_field))]
pub fn derive_filters(input: TokenStream) -> TokenStream {
    queryweave_codegen::filters::<MyConventions>(input.into()).into()
}
```

Each derive you want gets such a function: `queryweave_codegen::filters`,
`merge_patch`, `merge_patch_nullable_fields`, `validate_non_nullable`,
`raw_value`, `columns`, `append_to_query`, `append_conditions`.
List the attributes each derive reads in its `#[proc_macro_derive]`, as
`queryweave-derive` does; its source is the reference.

## The rules you can change

`Conventions` has one function per rule. The naming ones are required (you
can delegate any of them to `DefaultConventions`); the rest have defaults.

| Function                               | Decides                                                                                       |
| -------------------------------------- | --------------------------------------------------------------------------------------------- |
| `filters_names`                        | The names of every type `Filters` generates for a struct, and whether the struct's name fits. |
| `filter_field`                         | A filter's name, from the field and the operator.                                             |
| `sort_variant`                         | A sortable field's variant in the sort enum.                                                  |
| `check_model_name`                     | Whether a model deriving `MergePatch` is named acceptably.                                    |
| `check_merge_patch_name`               | Whether a merge patch struct is named acceptably.                                             |
| `check_wrapper_name`                   | Whether the filters wrapper named by `AppendToQuery` is named acceptably.                     |
| `core_crate`, `api_crate`, `sql_crate` | Where the generated code finds the runtime crates (default: `::queryweave`, ...).             |
| `integrations`                         | The integrations a mirror may opt into (default: the built-in ones).                          |

Report a name that does not fit with `queryweave_codegen::errors`
(`abort_spanned(span, message, help)`), at the span you are given: the
struct's name, or the path inside the attribute. `queryweave_codegen::names`
has the building blocks the default rules are made of (splitting a name into
words, checking a prefix), for rules close to them.

## What must agree across layers

Generated **type** names are local to each layer: layers refer to each
other's types through the paths you write (`mirrors = ...`), never by
rebuilding a name. So a domain crate and an API crate can use different
conventions for type names.

Generated **member** names are shared: a mirror names the root model's filter
fields and sort variants by applying its own `filter_field` and
`sort_variant` to the model's field names. Every layer of a chain therefore
has to agree on those two functions. If they do not, the build fails at the
mirror's field.

## Runtime crates

If your project re-exports the runtime crates from a crate of its own (for
example so that layers depend on one internal crate), point the conventions
at it:

```rust,ignore
fn core_crate() -> syn::Path {
    syn::parse_quote!(::my_platform::queryweave)
}
```

A single struct can also override it with `#[queryweave(crate = path)]`.

## A complete example

`queryweave-custom-conventions` in this repository is a proc-macro crate with
conventions unlike the default ones: whole-name type names, operator-first
filter names (`sw_name`), merge patch names containing `Patch`, runtime crates
re-exported under the user's modules, and an integration of its own. Its
tests check that every rule reaches the generated code.
