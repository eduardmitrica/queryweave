# Integrations

The types a mirror generates (its query parameters and sort types) usually
need to work with other libraries: a serializer to read them from a request,
a documentation generator to describe them, a validator to check them.
queryweave calls these **integrations**, and each mirror chooses its own.

## Choosing them

Integrations are flags of the mirror's `#[filters(...)]` attribute:

```rust,ignore
#[derive(Filters)]
#[filters(mirrors = domain::UserModel, serde, utoipa, validator)]
pub struct UserDto { ... }
```

A mirror without flags is plain: it gets the query parameters, sort types and
conversions, and depends on nothing but `queryweave`. You build its query
parameters yourself, which suits a layer whose input does not come from a
serializer, such as a gRPC message converted by hand.

The choice is per struct, not a Cargo feature of `queryweave-derive`. Cargo
merges the features a crate is used with across the whole build, so a feature
enabled for your API layer would also be enabled for every other layer. With
flags, a gRPC layer mirroring the same model does not pick up the API's
libraries.

## The built-in integrations

| Flag        | What the mirror's types get                                                                                                  | What you need                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `serde`     | `Deserialize` on the query parameters, `Deserialize` and `Serialize` on the sort types, and the page alias (`UsersPageDto`). | `serde`, and `queryweave-api` with its `serde` feature       |
| `utoipa`    | `IntoParams` and `ToSchema`, for OpenAPI documentation; `sort` is documented as a deep object.                               | `utoipa`, and `queryweave-api` with its `utoipa` feature     |
| `validator` | `Validate`: `page_size` must be at least 1, and `sort` must name each sortable field at most once.                           | `validator`; also the `serde` flag                           |
| `schemars`  | `JsonSchema`, the JSON schemas `aide`, `okapi` or `dropshot` build their documentation from.                                 | `schemars`, and `queryweave-api` with its `schemars` feature |
| `garde`     | `garde::Validate`, with the same rules as `validator`.                                                                       | `garde`                                                      |

Some notes on how they combine:

- `validator` needs `serde` because a validation error carries the rejected
  value, which `validator` serializes. `garde` has no such requirement.
- `serde`, `utoipa` and `schemars` use `queryweave-api`'s `SortDirectionDto`
  for the sort direction. Without them, the mirror uses the core
  `SortDirectionModel`. Mirrors of either kind chain into each other: the two
  directions convert both ways.
- Each integration states the format clients see itself (sort fields in snake
  case, such as `full_name`), rather than relying on another's attributes. The
  documentation from `utoipa` or `schemars` matches what `serde` reads, even
  if you use one without the other.

### `queryweave-api`'s features

`queryweave-api` holds the page and sort direction types the `serde`, `utoipa`
and `schemars` integrations use. It has one feature per integration, none
enabled by default, each adding that library's derives to those types:

```toml
queryweave-api = { version = "0.1", features = ["serde", "utoipa"] }
```

If you use a flag whose feature is not enabled, the build fails with an error
at the flag, naming the feature to enable:

```text
error[E0277]: the `serde` integration needs `queryweave-api`'s `serde` feature
  --> src/dtos/user.rs:11:32
   |
11 | #[filters(mirrors = UserModel, serde)]
   |                                ^^^^^ `queryweave-api` is built without its `serde` feature
   = note: enable it: `queryweave-api = { version = "..", features = ["serde"] }`
```

The compiler may follow it with errors about traits missing on
`SortDirectionDto`; they have the same cause.

## On `ValidateNonNullable`

The same flags, from the same list, choose the validation library of a merge
patch DTO: `#[validate_non_nullable(mirrors = path, validator)]` or `garde`.
Only validation libraries apply there; see
[guide.md](guide.md#validatenonnullable-on-the-apis-patch-type).

## Writing your own

If you need an integration queryweave does not ship, for example another
serializer or a GraphQL library, you can write it, as part of a proc-macro
crate of your own (see [conventions.md](conventions.md) for how to set one
up).

An integration is a type implementing
`queryweave_codegen::integrations::Integration`. Every method has a default
that adds nothing, so you implement only what your integration needs:

| Method                                                    | What it adds                                                                                                                             |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `name`                                                    | The flag users write (required).                                                                                                         |
| `requires`                                                | Other integrations it cannot work without, with the reason, reported at its flag when missing.                                           |
| `names_api_runtime`                                       | Whether its code uses `queryweave-api`'s types, so the mirror names that crate.                                                          |
| `sort_direction`                                          | The sort direction type it needs, if not the core `SortDirectionModel`.                                                                  |
| `page`                                                    | The type of a page of results, for the page alias.                                                                                       |
| `sort_field`, `sort_details`, `query_params`              | Derives and attributes for each of those generated types.                                                                                |
| `page_size`, `sort`                                       | Attributes on those query parameters.                                                                                                    |
| `items`, `sortable_items`                                 | Extra items: trait impls, helper functions (the latter only for mirrors with sortable fields).                                           |
| `supports_validate_non_nullable`, `validate_non_nullable` | For a validation library: that it applies to `ValidateNonNullable`, and the DTO's `validate_non_nullable()` built on `rejected_nulls()`. |

For example, an integration deriving a GraphQL input type:

```rust,ignore
use queryweave_codegen::integrations::{Context, Integration, TypeAdditions};
use quote::quote;

struct Graphql;

impl Integration for Graphql {
    fn name(&self) -> &'static str {
        "graphql"
    }

    // A direction type of your own, deriving the GraphQL enum and converting
    // both ways with `queryweave::SortDirectionModel`.
    fn sort_direction(&self, _context: &Context<'_>) -> Option<proc_macro2::TokenStream> {
        Some(quote!(crate::graphql::SortDirection))
    }

    fn sort_field(&self, _context: &Context<'_>) -> TypeAdditions {
        TypeAdditions {
            derives: vec![quote!(async_graphql::Enum)],
            attributes: quote!(#[graphql(rename_items = "snake_case")]),
        }
    }
}
```

Register it next to the built-in ones in your conventions:

```rust,ignore
fn integrations() -> Vec<&'static dyn Integration> {
    queryweave_codegen::integrations::BUILT_IN
        .into_iter()
        .chain([&Graphql as &'static dyn Integration])
        .collect()
}
```

Leaving some built-in integrations out of the list removes their flags, for
example if you want to replace `serde` with another serializer.

Some rules keep integrations working together:

- Name the types your additions use by an absolute path (`::my_crate::...`) or
  a path from the user's crate (`crate::...`), never assuming what the user
  imported.
- State the format clients see in your own attributes, rather than reading
  another library's.
- If two integrations of one mirror provide different sort directions or page
  types, the build fails at the second one's flag.

`queryweave-custom-conventions` in this repository registers `wire`, a toy
serializer with its own sort direction and page types, and tests it end to
end.
