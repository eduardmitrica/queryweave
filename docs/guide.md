# Guide

This guide goes through every derive queryweave provides: what you write,
what it generates, and what the compiler checks for you. It follows a request
through an application from the inside out: the domain model first, then the
layers that mirror it, then the database, then partial updates.

The examples use a `User` with a username and a full name. All generated
names follow the default conventions; [conventions.md](conventions.md)
explains how to change them.

## 1. The root model: `Filters`

The root model is the one every other layer refers to, usually your domain
model. Mark each field with what clients may do with it:

```rust,ignore
use queryweave_derive::Filters;

#[derive(Filters)]
pub struct UserModel {
    pub id: UserId,

    #[filter(sw, c, ew)]
    #[sortable]
    pub username: Username<String>,

    #[filter(eq)]
    pub age: u32,

    #[filter(bool)]
    pub active: bool,
}
```

### Filters

`#[filter(...)]` lists the operators a field can be filtered with. Each
operator becomes one filter, named after the field and the operator:

| Operator | Meaning            | Applies to               | Filter name   |
| -------- | ------------------ | ------------------------ | ------------- |
| `eq`     | equal to           | any field                | `age_eq`      |
| `lt`     | less than          | numbers, dates and times | `age_lt`      |
| `gt`     | greater than       | numbers, dates and times | `age_gt`      |
| `sw`     | starts with        | text                     | `username_sw` |
| `c`      | contains           | text                     | `username_c`  |
| `ew`     | ends with          | text                     | `username_ew` |
| `bool`   | is true / is false | `bool`                   | `active`      |

Using an operator on a type it does not apply to (`sw` on a number, `lt` on
text) is a compile error at the field, explaining which types it accepts.

### Sorting

`#[sortable]` lets clients sort by the field, ascending or descending. A field
can be sortable without being filterable, and the other way round.

### Field types and value objects

A field's type has to say what raw value it holds, the one filters compare
with and the database stores. Numbers, `bool`, `String` and the `chrono`
date and time types are their own raw value, and `Option<T>` has the raw value
of `T`.

A value object, a newtype around one of those, gets its raw value from
`#[derive(RawValue)]`:

```rust,ignore
#[derive(RawValue)]
pub struct Username<T = String>(T);
```

A field whose type has no raw value is a compile error at the field.

### What you get

For `UserModel`:

| Type                      | What it is                                                                                                          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `UsersQueryModel`         | A query: `page`, `page_size`, `sort` and the filters (`and`). This is what your services pass around.               |
| `UsersFilterOptionsModel` | The filters alone: one `Option` per filter (`username_sw`, `age_eq`, ...). `has_options()` says whether any is set. |
| `UserSortFieldModel`      | An enum of the sortable fields (`Username`).                                                                        |
| `UserSortDetailsModel`    | One sort instruction: a field and a direction (`SortDirectionModel::Ascending` / `Descending`).                     |

These types hold plain data. They do not know about HTTP, JSON or SQL, so
your domain crate depends only on `queryweave`. The persistence layer reads
the query through `conditions()` (the filters that are set) and `sort_keys()`
(the requested order), which the SQL derives use for you.

## 2. Layers mirroring it: `Filters` with `mirrors`

Any other layer that receives queries, such as an HTTP API, mirrors the root
model:

```rust,ignore
#[derive(Filters)]
#[filters(mirrors = domain::models::user::UserModel, serde, utoipa, validator)]
pub struct UserDto {
    #[filter(sw, c, ew)]
    #[sortable]
    pub username: String,

    #[filter(eq)]
    #[equivalent_field(age)]
    pub age_in_years: u32,
}
```

- `mirrors = path` names the struct this one mirrors.
- The other flags (`serde`, `utoipa`, `validator`, ...) choose which libraries
  the generated types work with. Without them the mirror depends on nothing
  but `queryweave`. See [integrations.md](integrations.md).
- A field named differently from the model's maps onto it with
  `#[equivalent_field(name)]`. Clients keep using the mirror's name
  (`age_in_years_eq`); the query uses the model's (`age_eq`).

Keep the attributes in sync with the model's: the mirror converts into the
model's types field by field, so a filter or sortable field the model does
not have is a compile error at that field, not a runtime mismatch.

### What you get

For `UserDto`:

| Type                    | What it is                                                                         |
| ----------------------- | ---------------------------------------------------------------------------------- |
| `UsersQueryParams`      | The query parameters: `page`, `page_size`, `sort` and one field per filter.        |
| `UserSortFieldDto`      | The sortable fields, as clients write them (`username`).                           |
| `UserSortDetailsDto`    | One sort instruction from a client.                                                |
| `UsersFilterOptionsDto` | The filters alone, for a struct mirroring this one in turn.                        |
| `UsersPageDto`          | With `serde`: a page of `UserDto`s to return (`queryweave_api::PageDto<UserDto>`). |

A mirror without sortable fields has no `sort` parameter at all.

### Converting

The query parameters convert into the model's query with `.into()`:

```rust,ignore
async fn list_users(Query(params): Query<UsersQueryParams>) -> ... {
    let query: UsersQueryModel = params.into();
    ...
}
```

### Chains of mirrors

A mirror can mirror another mirror. A gRPC message can mirror the API's DTO,
which mirrors the domain model; each one maps its names onto the struct it
names directly. Convert one step at a time with `.into()`, or straight to the
root model's query:

```rust,ignore
use queryweave::IntoRootQuery;

let query: UsersQueryModel = message_params.into_root_query();
```

Every chain has to end at a root model. A struct mirroring itself is an error
at the path. Two structs mirroring each other fail to compile with an
`overflow evaluating the requirement <UserDto as Filterable>::Root` error
pointing at both `mirrors` paths: read it as "this chain of mirrors never
reaches a root model".

## 3. The database: `Columns`, `AppendToQuery`, `AppendToQueryWithSeparator`

The persistence layer maps the model's fields onto the columns of its entity,
then builds SQL from the query.

### `Columns` on the entity

```rust,ignore
#[derive(sqlx::FromRow, Columns)]
#[columns(model = UserModel)]
pub struct User {
    pub id: i32,
    pub username: String,

    #[domain_field(age)]
    pub age_years: i32,

    #[sqlx(rename = "is_active")]
    pub active: bool,
}
```

- `#[columns(model = path)]` names the domain model the entity stores.
- Each field is a column, named after the field, or after its
  `#[sqlx(rename = "...")]`.
- A column storing a domain field of another name says so with
  `#[domain_field(name)]`; that name must exist on the model.
- `#[sqlx(skip)]` and `#[sqlx(flatten)]` fields are not columns.

This is the only place a domain field name becomes a column name. Every SQL
helper checks, when your code is built, that each domain field it writes has a
column here, so a missing or renamed column fails the build instead of a
query.

### The query wrappers

Two newtypes around the generated domain types build the clauses:

```rust,ignore
#[derive(AppendToQueryWithSeparator)]
#[append_to_query(columns = User)]
pub struct UsersFilterOptionsModelWrapper(pub UsersFilterOptionsModel);

#[derive(AppendToQuery)]
#[append_to_query(columns = User, filters = UsersFilterOptionsModelWrapper)]
pub struct UsersQueryModelWrapper(pub UsersQueryModel);
```

- `AppendToQueryWithSeparator` writes the filters, joined by a separator
  (`"AND"`), for example after your own `WHERE`, or in a `COUNT(*)` query.
- `AppendToQuery` writes a whole query's clauses: `WHERE` (through the
  filters wrapper named by `filters`), `ORDER BY` and `LIMIT`/`OFFSET`.

In a repository:

```rust,ignore
let mut query = QueryBuilder::<Postgres>::new("SELECT * FROM users ");
let guard: Option<fn(&mut QueryBuilder<Postgres>, &UsersQueryModelWrapper)> = None;
UsersQueryModelWrapper(filters).append_to_query(&mut query, guard);
let users = query.build_query_as::<User>().fetch_all(&pool).await?;
```

The optional guard runs between the filters and the sorting, to add
conditions of your own (access rules, soft deletion).

The generated SQL is standard and portable across Postgres, MySQL/MariaDB
and SQLite. Every value is bound as a parameter, never written into the SQL,
and `LIKE` patterns escape the user's `%` and `_`.

## 4. Partial updates: `MergePatch`, `MergePatchNullableFields`, `ValidateNonNullable`

These handle `PATCH` requests with JSON merge patch semantics (RFC 7396): a
field left out of the request is unchanged, a field with a value is replaced,
and a field set to `null` is cleared, if it may be.

They are behind the `merge-patch` feature, on by default in every queryweave
crate. For filtering, sorting and paging alone, turn the default features off
(`default-features = false`) on each queryweave crate you use.

### `Patch<T>`: the only way to say "absent, null or a value"

A field of a patch is one of two types:

| Type                   | The client may                             | Use it for                     |
| ---------------------- | ------------------------------------------ | ------------------------------ |
| `Option<T>`            | leave it out, or send a value              | a field that cannot be cleared |
| `queryweave::Patch<T>` | leave it out, send `null`, or send a value | a field that can be cleared    |

`Patch<T>` has three states, `Patch::Missing`, `Patch::Null` and
`Patch::Value(t)`, and `Missing` is its default.

`Option<Option<T>>` is **not** supported. It is a common way to encode the
same three states, but queryweave does not accept it anywhere: the derives
reject it at the field with an error naming `Patch<T>`. Accepting it would
be a trap, since an `Option<Option<T>>` field also reads as an `Option<P>`
field that cannot be cleared, and a `null` sent for it would be silently
ignored.

### The domain's patch model

```rust,ignore
use queryweave::Patch;

#[derive(MergePatchNullableFields)]
pub struct MergePatchUserModel {
    #[merge_patch(skip)]
    pub id: UserId,

    pub username: Option<Username<String>>,
    pub nickname: Patch<String>,
}
```

`MergePatchNullableFields` records which fields may be set to `null`: the
`Patch` ones (`nickname` here). A field that is not part of the patch at all,
such as an id, is marked `#[merge_patch(skip)]`. Under the default
conventions, a patch model's name starts with `MergePatch`.

### `MergePatch` on the model

```rust,ignore
#[derive(MergePatch)]
#[merge_patch(patch = MergePatchUserModel)]
pub struct UserModel {
    #[merge_patch(skip)]
    pub id: UserId,

    pub username: Username<String>,
    pub nickname: Option<String>,
}
```

- `#[merge_patch(patch = path)]` names the patch model, which must derive
  `MergePatchNullableFields` (so its fields are checked).
- Every field not marked `#[merge_patch(skip)]` must exist on the patch as an
  `Option<P>`, or as a `Patch<P>` if the model's field is an `Option` that
  may be cleared, with `P` converting into the field's type.

You get `merge(patch)`, which applies a patch; `modified_fields(&before)`
(from the `queryweave::ChangedFields` trait), which lists the fields that
actually changed; and `changed_values(&changed)`, which gives their new values
for the database:

```rust,ignore
use queryweave::ChangedFields;

let before = user.clone();
user.merge(patch);
let changed = user.modified_fields(&before);

if !changed.is_empty() {
    let mut query = QueryBuilder::<Postgres>::new("UPDATE users SET ");
    User::push_assignments(&mut query, user.changed_values(&changed));
    query.push(" WHERE id = ").push_bind(id);
    ...
}
```

`push_assignments`, generated on the entity by `Columns`, writes the `SET`
clause, and checks at build time that every patchable field has a column.

### `ValidateNonNullable` on the API's patch type

The API's patch type receives the request body. Every field is a `Patch<T>`,
so that a `null` can be seen at all, and `ValidateNonNullable` rejects it for
each field the domain does not allow to be cleared:

```rust,ignore
use queryweave::Patch;

#[derive(Default, Deserialize, ToSchema, Validate, ValidateNonNullable)]
#[validate_non_nullable(mirrors = MergePatchUserModel, validator)]
pub struct MergePatchUserDto {
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub username: Patch<String>,

    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub nickname: Patch<String>,
}

dto.validate_non_nullable()?; // `{"username": null}` is an error, `{"nickname": null}` is not
```

- `Patch<T>` deserializes with `queryweave-api`'s `serde` feature (which
  turns on the core's). Its `schemars` feature gives it a JSON schema, and
  its `validator` feature lets validator's `range` rule check a `Patch`
  field (a value when one is given), so the DTO keeps rules such as
  `#[validate(range(min = 0))]`.
- `#[serde(default)]` on each field makes an absent field `Patch::Missing`.
  Put it on the fields rather than the struct: utoipa documents a
  struct-level default as `"default": null`, which is not what an absent
  field means.
- `#[schema(value_type = Option<T>)]` documents the field as the nullable
  value clients send. utoipa composes generic field types through an
  internal trait, so this one attribute per field is needed.

`ValidateNonNullable` itself names no validation library. It always
generates `rejected_nulls()`, the DTO's fields set to `null` that may not be,
by the names clients send. A flag picks the validation library whose errors
`validate_non_nullable()` returns:

| Flag        | `validate_non_nullable()` returns            | Runs first                     |
| ----------- | -------------------------------------------- | ------------------------------ |
| _(none)_    | not generated: use `rejected_nulls()` itself | -                              |
| `validator` | `Result<(), validator::ValidationErrors>`    | the struct's `Validate`        |
| `garde`     | `Result<(), garde::Report>`                  | the struct's `garde::Validate` |

It runs the struct's own rules first, then reports each rejected `null`
under the DTO's field name. One validation library at a time: a second
flag, or one that is not a validation library, is an error at the flag. A field named
differently from the domain patch's maps onto it with
`#[equivalent_field(name)]`. Like `Filters` mirrors, these chain: a gRPC patch
message can mirror the API's patch DTO.

## Renaming a generated type

The conventions name every generated type. To rename one of them for a single
struct, without writing conventions of your own, add its option to
`#[filters(...)]`:

```rust,ignore
#[derive(Filters)]
#[filters(query = UserSearch, sort_field = UserSortKey)]
pub struct UserModel { ... }

#[derive(Filters)]
#[filters(mirrors = UserModel, serde, query_params = UserSearchParams)]
pub struct UserDto { ... }
```

| Option           | Renames                             | On       |
| ---------------- | ----------------------------------- | -------- |
| `query`          | the query model (`UsersQueryModel`) | a root   |
| `query_params`   | the query parameters                | a mirror |
| `filter_options` | the filter options                  | both     |
| `sort_field`     | the sortable fields enum            | both     |
| `sort_details`   | a sort instruction                  | both     |
| `page`           | the page alias                      | a mirror |

The other types keep their conventions' names, and layers still link through
the paths you write, so renaming a root's type does not affect its mirrors.
A rename of a type the struct does not generate, and two generated types (or
one and the struct) sharing a name, are errors at the name.

## Where the runtime crates live

The generated code refers to the runtime crates by their usual names
(`::queryweave`, `::queryweave_api`, `::queryweave_sql`). If you renamed one,
or re-export it from a crate of your own, point a struct at it:

```rust,ignore
#[derive(Filters)]
#[queryweave(crate = my_platform::queryweave)]
pub struct UserModel { ... }
```

Every derive accepts `#[queryweave(crate = path)]`. To change it for a whole
project, set it in your conventions instead (see
[conventions.md](conventions.md)).

## Errors

queryweave reports mistakes at compile time, pointing at what you wrote:

- an unknown, repeated or malformed attribute option, at the option;
- a filter operator on a type it does not apply to, at the field;
- a mirror field the mirrored struct does not have, at the field (or the name
  in its `#[equivalent_field]`);
- a struct linking to itself, at the path;
- a domain field reaching SQL without a column, at the call writing it;
- an integration whose library feature is not enabled, at the integration's
  flag (see [integrations.md](integrations.md));
- an `Option<Option<T>>` merge patch field, at its type, naming `Patch<T>`;
- a patch model that does not derive `MergePatchNullableFields`, at the
  `patch = ...` path;
- two generated types sharing a name, at the second name;
- queryweave crates of incompatible versions, at each derive (use the same
  version of every queryweave crate).
