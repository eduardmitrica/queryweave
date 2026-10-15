# queryweave

queryweave generates the code that lets the clients of an application filter,
sort, page through and partially update its data, from a few attributes on
your domain model.

The SQL is built on [sqlx](https://docs.rs/sqlx),
[sea-query](https://docs.rs/sea-query) (and so
[SeaORM](https://docs.rs/sea-orm)) or [Diesel](https://docs.rs/diesel), for Postgres, MySQL/MariaDB and SQLite. The query parameters work with the libraries you choose: with
serde, they deserialize from a query string read by
[serde_qs](https://docs.rs/serde_qs).

## The problem it solves

Take a list endpoint: `GET /users?username_sw=ad&page=2&page_size=20`, which
returns the second page of users whose username starts with "ad", in some
requested order.
To serve it, a layered application usually writes the same idea several
times:

- the **API layer** needs query parameters to deserialize, document and
  validate;
- the **domain** needs a query type that the application logic can pass
  around without knowing about HTTP or SQL;
- the **persistence layer** needs to turn that query into `WHERE`,
  `ORDER BY` and `LIMIT`/`OFFSET` clauses.

Written by hand, these copies drift apart: a field filterable in the API but
not in the database, a renamed column that only fails at runtime, a sort
option documented but never applied. queryweave writes all three from one
description, and checks at compile time that they agree.

## How it works

You mark the fields of a **root model**, usually your domain model, with what
can be done with them:

```rust,ignore
use queryweave_derive::Filters;

#[derive(Filters)]
pub struct UserModel {
    pub id: UserId,

    #[filter(sw, c, ew)] // starts with, contains, ends with
    #[sortable]
    pub username: Username<String>,
}
```

From this, the domain gets a query type, `UsersQueryModel`, holding the
requested page, sort order and filters (`username_sw`, `username_c`,
`username_ew`), as plain data with no database or serializer attached.

Every other layer **mirrors** the root model. A mirror lists the same fields,
under the names that layer uses, and says which libraries its types should
work with:

```rust,ignore
#[derive(Filters)]
#[filters(mirrors = domain::UserModel, serde, utoipa, validator)]
pub struct UserDto {
    #[filter(sw, c, ew)]
    #[sortable]
    pub username: String,
}
```

The API layer now has `UsersQueryParams`, which deserializes from the query
string, is documented in the OpenAPI schema and validates the page size and
sort, and converts into the domain's query with `.into()`. A field the API
names differently maps onto the model's with `#[equivalent_field(name)]`, and
a name the model does not have is a compile error at that field.

The filters and the page are flat parameters, but `sort` is a list of fields
and orders, which a query string writes with brackets:
`?username_sw=ad&sort[0][field]=username&sort[0][order]=ascending`. With the
`serde` flag, read it with [serde_qs](https://docs.rs/serde_qs), which
understands that notation; `serde_urlencoded`, which axum's own `Query`
extractor uses, does not. A mirror using another serializer needs a reader
of its own that handles a list of structs.

The persistence layer maps the model's fields onto the columns of its
entity, and gets the SQL from that:

```rust,ignore
#[derive(sqlx::FromRow, Columns)]
#[columns(model = UserModel)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(AppendConditions)]
#[append_conditions(columns = User)]
pub struct UsersFilterOptionsModelWrapper(pub UsersFilterOptionsModel);

#[derive(AppendToQuery)]
#[append_to_query(columns = User, filters = UsersFilterOptionsModelWrapper)]
pub struct UsersQueryModelWrapper(pub UsersQueryModel);

// In the repository: adds the WHERE, ORDER BY and LIMIT/OFFSET clauses.
let mut query = QueryBuilder::<Postgres>::new("SELECT * FROM users ");
// An optional hook to add conditions of your own (e.g. access rules); none here.
let guard: Option<fn(&mut QueryBuilder<Postgres>, &UsersQueryModelWrapper)> = None;
UsersQueryModelWrapper(filters).append_to_query(&mut query, guard);
```

Every value is bound, never interpolated, and a domain field that reaches
SQL without a column fails the build.

Partial updates (`PATCH` with JSON merge patch semantics) work the same way.
A patch says, for each field, whether it is left out, cleared or replaced.
A field that can be cleared is a `queryweave::Patch<T>` (`Missing`, `Null` or
`Value(t)`); one that cannot is an `Option<T>`. `Option<Option<T>>` is not
supported, and the derives reject it with an error naming `Patch<T>`.
`MergePatch` on the model merges a patch into it and reports which fields
changed, `Columns` writes those fields as a `SET` clause, and
`ValidateNonNullable` on the API's patch type rejects `null` for fields that
cannot be cleared.

Merge patches are behind the `merge-patch` feature, on by default. Turn the
default features off on the queryweave crates to use the filters alone (and
keep `queryweave-sql`'s `sqlx` feature: `features = ["sqlx"]`).

## The crates

You depend on `queryweave-derive` for the macros, and on one runtime crate
per layer, which holds the types the generated code uses:

| Crate                | Use it in                   | What it holds                                                                                                                                                                                                                                     |
| -------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `queryweave-derive`  | every layer                 | The derive macros.                                                                                                                                                                                                                                |
| `queryweave`         | the domain                  | The core: query, sort and patch data. No database or serializer dependency, so the domain stays free of both.                                                                                                                                     |
| `queryweave-api`     | the API layer               | Pages and sort directions for API types, with optional `serde`, `utoipa` and `schemars` support.                                                                                                                                                  |
| `queryweave-sql`     | the persistence layer       | The SQL, written into sqlx's `QueryBuilder` (feature `sqlx`, default) or sea-query's / SeaORM's statements (feature `sea-query`), or as Diesel's typed expressions (feature `diesel`), for Postgres, MySQL/MariaDB, SQLite. No driver dependency. |
| `queryweave-codegen` | a proc-macro crate of yours | The code generation itself, for when you want your own naming rules or integrations (see below).                                                                                                                                                  |

`queryweave-api` and `queryweave-sql` re-export the core, so each layer needs
only its own runtime crate.

## Integrations

A mirror only depends on the libraries it asks for. Without flags it depends
on nothing but `queryweave`; each flag adds what that library needs:

| Flag        | What the mirror's types get                                                                      |
| ----------- | ------------------------------------------------------------------------------------------------ |
| `serde`     | Deserializable query parameters, and a page type for responses.                                  |
| `utoipa`    | OpenAPI documentation.                                                                           |
| `validator` | Validation of the page size and sort (it reports errors through serde, so it needs `serde` too). |
| `schemars`  | JSON schemas, as used by `aide`, `okapi` or `dropshot`.                                          |
| `garde`     | Validation of the page size and sort, without a serializer.                                      |

Each layer picks its own flags, so a gRPC layer mirroring the same model does
not pull in the API layer's libraries. See [docs/integrations.md](https://github.com/eduardmitrica/queryweave/blob/main/docs/integrations.md).

## Your own rules

The names queryweave generates follow conventions: `UserModel` gets
`UsersQueryModel`, `UserDto` gets `UsersQueryParams`, a filter on `username`
with `sw` is `username_sw`. If your project names things differently, or
needs an integration queryweave does not ship (another serializer, GraphQL),
you write a small proc-macro crate of your own on top of `queryweave-codegen`
with your conventions. See [docs/conventions.md](https://github.com/eduardmitrica/queryweave/blob/main/docs/conventions.md).

## Documentation

- [docs/guide.md](https://github.com/eduardmitrica/queryweave/blob/main/docs/guide.md): every derive, what you write and what you get.
- [docs/integrations.md](https://github.com/eduardmitrica/queryweave/blob/main/docs/integrations.md): the integration flags, and writing your own.
- [docs/conventions.md](https://github.com/eduardmitrica/queryweave/blob/main/docs/conventions.md): naming rules and runtime paths of your own.
- [docs/architecture.md](https://github.com/eduardmitrica/queryweave/blob/main/docs/architecture.md): how it works inside, for contributors.
- [docs/ai/architecture-reference.md](https://github.com/eduardmitrica/queryweave/blob/main/docs/ai/architecture-reference.md): a condensed reference for AI agents.

## Development

queryweave is a Cargo workspace of its own. From its root, one script runs
every check:

```shell
sh scripts/check.sh        # formatting, lints, unit and doc tests, rustdoc,
                           # every queryweave-api feature combination and the
                           # filters-only build; needs no Docker
sh scripts/check.sh ci     # the same, plus the database tests (Postgres,
                           # MySQL and MariaDB in containers: needs Docker)
sh scripts/check.sh all    # the same, plus the trybuild UI tests, which are slow
                           # (each case is a full cargo build)
```

The tests needing neither Docker nor a full build per case run with
`cargo nextest run --workspace --profile fast`.

### Pre-commit hook

`pre-commit.sh` runs `scripts/check.sh fast` before each commit. To install it
as the repository's git hook, build `pre_commit_script_builder` (a small binary
that runs the script) and copy it into `.git/hooks`:

```shell
cd pre_commit_script_builder && cargo build --release && cd ..
cp pre_commit_script_builder/target/release/pre-commit .git/hooks/pre-commit
# Windows: copy pre-commit.exe to .git\hooks\pre-commit.exe instead
```

Rebuilding is only needed when the builder changes: edits to `pre-commit.sh`
or `scripts/check.sh` apply right away.

## Status

queryweave is published on crates.io. Depend on `queryweave-derive` and on the
runtime crate of each layer (see [The crates](#the-crates)):

```toml
[dependencies]
queryweave-derive = "0.1"
queryweave = "0.1" # or `queryweave-api` / `queryweave-sql`, per layer
```

Its version is `0.1.0`: the API may still change before `1.0`.

## Minimum supported Rust version

Rust 1.88, with every feature. SeaORM 2.0 itself needs Rust 1.94, so a project
writing into SeaORM's queries (through the `sea-query` feature) needs that
version for SeaORM's sake.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/eduardmitrica/queryweave/blob/main/LICENSE-APACHE) or
[MIT license](https://github.com/eduardmitrica/queryweave/blob/main/LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this crate by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
