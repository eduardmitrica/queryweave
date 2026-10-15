# queryweave: notes for agents

queryweave is a set of **derive procedural macros** that generate the filtering, sorting,
pagination and merge-patch plumbing of a layered (clean / hexagonal) Rust application from
attributes on its types: the domain models, the API DTOs and the persistence entities. Every layer
gets its own generated types, linked to each other at compile time, so a renamed or missing field is
a compile error at that field rather than a runtime string mismatch. SQL is generated portably for
Postgres, MySQL/MariaDB and SQLite through sqlx.

This file is the condensed working guide for agents (and the source a `CLAUDE.md` is generated
from). The explanations for people are in `README.md` and `docs/`; see [Docs](#docs).

- Repository: <https://github.com/eduardmitrica/queryweave>, default branch `main`, CI on GitHub
  Actions (`.github/workflows/ci.yml`).
- Version `0.1.0`, the five published crates in lockstep; licensed `MIT OR Apache-2.0`
  (`LICENSE-MIT` / `LICENSE-APACHE` at the root and in each published crate); MSRV 1.88 (for the default and every optional feature; SeaORM itself, only a dev-dependency, needs 1.94); one shared
  `README.md` is each crate's readme; `repository` comes from `[workspace.package]`. Published
  on crates.io (`cargo publish --workspace`, the five crates together); the README's links are
  absolute GitHub URLs, since it is rendered on each crate's crates.io page.
- A dependency between queryweave crates carries `version` next to `path` (needed to publish).
- It was extracted (issue #54) from the **rust-axum-sqlx** boilerplate, which remains its reference
  consumer: a domain / application / infrastructure / api layering over Axum + SQLx, depending on
  these crates from this repository. Examples below (`OrderModel`, `OrderDto`, `Order`) are its types.

## Working agreement

- **Never commit, push or stage.** The maintainer reviews and commits every change themselves;
  leave changes in the working tree and suggest a commit message (conventional commits:
  `feat:`, `fix:`, `perf:`, `chore:` ...).
- Large changes go one reviewable step at a time: finish a step, verify it, stop for review.
- Run `cargo fmt --all` **first**, before any test suite or clippy, so verification runs on formatted
  code and formatting never shows up as a separate change.
- Verification includes `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` (zero
  warnings): clippy does not catch rustdoc lints such as `rustdoc::private_intra_doc_links`. In
  public docs, name a private item as plain code (`` `item` ``), never as a link.
- No lint allows or hacks when a normal design does the job: generate docs instead of
  `allow(missing_docs)`, extract a helper instead of `allow(too_many_lines)`, rename instead of
  `allow(struct_field_names)`, use anonymous `const _: … = …;` blocks instead of named items on user
  types, and call-site spans for generated code that is not the user's (clippy lints tokens carrying
  user spans). An allow that remains is the narrowest scope that works and carries
  `reason = "..."`. The `#[doc(hidden)] pub mod __private` for macro-only APIs is the accepted idiom.
- One pattern everywhere: a helper that fits several derives goes in `queryweave-codegen`'s
  plumbing modules, and every hand-written equivalent is migrated in the same change (grep the
  other derives before finishing).
- Spike risky type-system designs (coherence, projections, trait solving) in a scratch crate before
  building them into the derives.

## Commands

All from the repository root.

| Purpose             | Command                                                                                       |
| ------------------- | --------------------------------------------------------------------------------------------- |
| Format              | `cargo fmt --all` (first, always)                                                             |
| Every check         | `sh scripts/check.sh fast` (no Docker) · `ci` (+ database tests: Docker) · `all` (+ UI tests) |
| Unit tests          | `cargo nextest run --workspace --profile fast`                                                |
| UI tests (trybuild) | `cargo nextest run --workspace -E 'test(/ui_tests/)'`                                         |
| Database tests      | `sh scripts/check.sh ci` (Docker: sqlx and Diesel on Postgres, MySQL and MariaDB)             |
| Doc tests           | `cargo test --workspace --doc`                                                                |
| Lints               | `cargo clippy --workspace --all-targets --all-features -- -D warnings`                        |
| Rustdoc             | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`                                  |
| Pre-commit hook     | `sh pre-commit.sh` (runs `scripts/check.sh fast`)                                             |
| MSRV (CI job)       | `cargo +1.88.0 check --locked -p <each published crate> --all-features` (libraries only)      |

`scripts/check.sh` is the single list of checks: formatting, lints, unit and doc tests, rustdoc,
`queryweave-api` alone under every feature combination (`""`, `serde`, `utoipa`, `schemars`,
`validator`, all four; clippy + tests + its feature UI cases, since the workspace build unifies every
feature), the filters-only build (`--no-default-features`, no `merge-patch`), then the database tests
(`ci`) and the UI tests (`all`). CI runs `check.sh ci` and the UI tests as separate jobs. The hook is
`pre-commit.sh`, built into a binary by `pre_commit_script_builder` (workspace-`exclude`d tooling;
install steps in the README).

nextest profiles (`.config/nextest.toml`): `fast` leaves out the UI tests and the database tests
(`queryweave-sql/tests/{postgres,mysql}.rs` for sqlx and `tests/diesel_{postgres,mysql}.rs` for
Diesel, which start Postgres, MySQL and MariaDB containers;
`tests/sqlite.rs` runs in memory, in `fast`); `ci` leaves out the UI tests; `default` runs
everything and serializes the UI tests in the `ui-tests` group, because every `test_ui_*` of a crate
builds through the same generated trybuild project and concurrent runs clobber each other's
manifest (dtolnay/trybuild#276). Each UI case is a full `cargo build`, so they are slow.

Regenerate expected `.stderr` files with `TRYBUILD=overwrite` and review the diff.

## Toolchain

`rust-toolchain.toml` pins 1.99.0 with `clippy`, `rustfmt`, `rust-analyzer` and **`rust-src`**.
`rust-src` is required, not optional: some `.stderr` files quote the standard library's source
(`$RUST/core/...`), which rustc only prints when `rust-src` is installed. Without it the UI tests
fail with `= note: the failure occurred here` in place of the quoted line.

Lints are strict for every crate (`[workspace.lints]`): `unsafe_code = "forbid"`, `missing_docs`,
clippy `all` + `pedantic` + `nursery` under `-D warnings`, and denied `unwrap_used`, `expect_used`,
`panic`, `indexing_slicing`, `wildcard_imports`. New public items need doc comments. `clippy.toml`
relaxes `unwrap` / `expect` / `panic` / indexing in tests and consts. rustfmt: 100 columns, Unix
newlines, edition 2024.

## Crates

| Crate                           | Owns                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `queryweave`                    | The core runtime, used by a consumer's domain layer and named by the code generated for root models: `SortDirectionModel`, `ChangeSet`, `ChangedFields`, `MergePatchNullableFields`, `Patch<T>`, `RawValue` + the filter kinds `Ordered` / `Text` / `Boolean`, `FilterOperator`, and the database-agnostic `Value` / `IntoValue` (+ `ValueKind` / `ValueType`, its compile-time type) / `FilterCondition` / `SortKey` / `FieldValue` the generated code hands to the persistence layer (they name domain fields, not columns), `ModelData<M, T>` (aliases `Conditions<M>` / `SortKeys<M>` / `FieldValues<M>`), `PersistedFields` (which domain fields a generated type's data names); `Filterable` (a `Filters` model's generated types, as associated types, incl. `Root`) and the role traits `QueryOf` / `FilterOptionsOf` / `SortFieldOf` / `SortDetailsOf` a mirror implements to convert into them; `NullableFields` (which fields of a merge patch struct may be set to `null`, under its own names); `require_version`. Features: `merge-patch` (default), `serde` / `schemars` / `validator` (on `Patch<T>` only). No database dependency and no SQL text.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `queryweave-api`                | The API runtime, named by the code generated for API types (a `Filters` mirror with `serde` / `utoipa` / `schemars`, `ValidateNonNullable`): `PageDto<T>`, `SortDirectionDto` (converting both ways with `SortDirectionModel`); glob re-exports the core. One feature per integration, none by default (`serde`, `utoipa`, `schemars`, `validator`; each adds its impls to `SortDirectionDto` / `PageDto` or turns on the core's `Patch<T>` feature, stating the snake case wire format itself), so with none it depends on nothing but the core; never sqlx. Its `features` module has a trait per feature (`features::Serde`, ...), implemented for `SortDirectionDto` only with the feature and carrying `#[diagnostic::on_unimplemented]`: a mirror's integration requires it in a check spanned to its flag (emitted first), so a disabled feature is reported at the flag, naming the feature to enable.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `queryweave-sql`                | The SQL runtime, named by the code generated for entities and wrappers (glob re-exports the core): `AppendToQuery`, `AppendConditions` (generic over the query); the `Columns` trait (domain field → column) and `assert_columns!`; all SQL for the domain's data, portable across Postgres, MySQL/MariaDB and SQLite (`BindValues`: any sqlx database binding the domain's raw values): `Columns::push_conditions` (filters → `WHERE`, incl. `LIKE … ESCAPE '!'`), `push_order_by` (`ORDER BY`), `push_assignments` (merge-patch writes → `SET`), each checking at compile time that every field of the model has a column, then writing into the query through its target trait (`ConditionsTarget` / `OrderByTarget` / `AssignmentsTarget` / `PageTarget`, implemented for sqlx's `QueryBuilder` with the `sqlx` feature, on by default, and for sea-query's `SelectStatement` / `UpdateStatement` / `DeleteStatement`, SeaORM's through `QueryTrait::query`, with `sea-query`; each trait's `#[diagnostic::on_unimplemented]` names the features; with `sea-query` also the `sea_query` module, `check_column_types` of a SeaORM entity's column types at run time); `Junction` (`And` / `Or`, joining conditions); `push_page` (`LIMIT` / `OFFSET`), `sql_operator`, `sql_direction`, `sql_junction` and the helpers for a target of one's own (`like_pattern`, `escape_like`, `LIKE_ESCAPE`, `page_offset`); with `diesel`, the `diesel` module (typed conditions, orderings, page and assignments over Diesel columns, `ValueMismatch`; `Entity`, a Diesel entity's table and methods for the wrappers; `apply` on a `BoxedQuery`). No driver dependency. |
| `queryweave-derive`             | The proc-macro crate exporting every derive under `DefaultConventions`, each one line forwarding to `queryweave-codegen`: `Filters` (a **root** model, or a **mirror** with `#[filters(mirrors = path)]`), `MergePatch`, `MergePatchNullableFields`, `RawValue`, `ValidateNonNullable`, `AppendToQuery`, `AppendConditions`, `Columns`. Holds the derives' behaviour tests and trybuild UI suites. No runtime dependencies (only its tests have them).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `queryweave-codegen`            | The code generation of every derive as an ordinary library, one generic function per derive (`queryweave_codegen::filters::<C>(TokenStream) -> TokenStream`, ...), for proc-macro crates with their own rules. Public API: the derive functions, `conventions`, `integrations`, `errors`, `names`. Private plumbing, free to change: `steps`, `attributes`, `checks`, `utils` and the step modules.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `queryweave-custom-conventions` | Test fixture (`publish = false`): a proc-macro crate on `queryweave-codegen` with non-default conventions (other type and filter-field names, re-exported runtime paths, an integration of its own, `wire`, a toy serializer other than serde), whose tests check every rule reaches the generated code.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

Every crate has the `merge-patch` feature (default); turning it off gives the filters alone, and
that build is checked. `queryweave-sql` also has a feature per query builder, `sqlx` (default) and
`sea-query`: each is a backend module behind a feature of its name, only adding target-trait impls;
and `diesel` (+ `diesel-postgres` / `diesel-mysql` / `diesel-sqlite`, mirroring Diesel's backend
features for each backend's date-time SQL types; never Diesel's `sqlite` on its own, which links
SQLite): the public `queryweave_sql::diesel` module, typed functions over Diesel columns (Diesel's
builders are by value and typed, so it implements no target trait). `check.sh` builds the crate with
each, all and none.

## Module map

- `queryweave-codegen/src`: `lib.rs` (one public function per derive, each
  `steps::expand::<pipeline::<derive>::<Derive><C>>(input)`, plus the plumbing modules), `steps.rs`
  (the `Derive` pipeline trait, `TopLevelMeta`, the `expand` driver), `pipeline/` (one
  `steps::Derive` impl per derive, generic over its conventions, wiring its steps), then one module
  per pipeline step with one file per derive inside each: `parse_top_level_meta/` → `mapping/`
  (every derive: at least its runtime crate) → `parse/` → `analyze/` → `lower/` → `code_gen/`.
  Simple derives skip steps (the infrastructure derives have no `analyze/` / `lower/`); a derive with
  roles splits its last step (`Filters`: `code_gen/filters/{root,mirror}.rs`, dispatching on the role
  `mapping/` decides). Plumbing: `attributes.rs` (`path_option` / `required_path_option` /
  `require_option` for `#[name(key = path)]`, `flag_option` for `#[name(key)]`, `crate_option` for
  `#[queryweave(crate = path)]`, `equivalent_field_name`), `checks.rs` (compile-time assertions for
  generated code, e.g. `assert_fields_exist`), `conventions.rs` (the `Conventions` trait and
  `DefaultConventions`), `integrations.rs` (the `Integration` hook, `BUILT_IN`) +
  `integrations/{serde,utoipa,validator,schemars,garde}.rs`, `errors.rs` (incl. the `Errors`
  collector and `both`), `names.rs` (struct-name checks: pascal case, minimum segments,
  `MergePatch…`, `path_name`), `utils.rs` (pascal-case splitting, `unwrap_option`, `respanned`). Unit
  tests (and their `insta` snapshots, under `snapshots/`) sit next to each step.
- `queryweave-derive`: `src/lib.rs` (the `#[proc_macro_derive]` entry points, each
  `queryweave_codegen::<derive>::<DefaultConventions>(input.into()).into()`), `tests/<derive>.rs`
  (generated-code behaviour), `tests/ui.rs` + `tests/ui/<derive>/{pass,fail}/` (trybuild).
- `queryweave/src`: `lib.rs` (shared types, `__private`), `model_data.rs`, `raw_value.rs`,
  `value.rs`, `filterable.rs`, `patch.rs`, `version.rs`; `tests/no_sqlx.rs`.
- `queryweave-api/src`: `lib.rs`, `page.rs`, `features.rs`; `tests/ui/features/` (feature UI cases).
- `queryweave-sql/src`: `target.rs` (the target traits, `Junction`), `backend.rs` (+ the shared
  `column` lookup) + `backend/sqlx.rs` (+ `backend/sqlx/bind.rs`, `BindValues`; behind the `sqlx`
  feature) + `backend/sea_query.rs` (behind `sea-query`, public as `queryweave_sql::sea_query`;
  `backend/sea_query/check.rs`, `check_column_types`) + `backend/diesel.rs` (behind `diesel`,
  public as `queryweave_sql::diesel`; `backend/diesel/check.rs`, the compile-time type check), `columns.rs`, `sql.rs` (the SQL every
  backend shares), `lib.rs` (wrapper traits, `__private`); `tests/{sqlite,postgres,mysql}.rs`
  (`required-features = ["sqlx"]`), `tests/sea_query_sqlite.rs` (`["sea-query"]`),
  `tests/diesel_sqlite.rs` (`["diesel-sqlite"]`), `tests/diesel_{postgres,mysql}.rs`
  (`["diesel-postgres"]` / `["diesel-mysql"]`, through `diesel-async`), `tests/ui.rs` +
  `tests/ui/features/` (feature UI cases; the failing ones checked without any backend, since rustc
  lists the enabled backends' impls).

## Hard rules (do not violate)

### Crate boundaries

- Code generation lives in `queryweave-codegen` (its pipeline, plus a public
  `queryweave_codegen::<derive>::<C>` function). A proc-macro crate only forwards to it
  (`queryweave-derive` with `DefaultConventions`, a crate with its own rules with its own
  conventions) and exports **only** derives.
- Any type or trait the generated code needs at runtime lives in a runtime crate, so both the macro
  and its consumer can name it: `queryweave` for a root model's and the role traits,
  `queryweave-api` for API types, `queryweave-sql` for SQL.
- Each derive's generated code names exactly **one** runtime crate, the one of its layer
  (`queryweave-api` and `queryweave-sql` glob re-export the core, so they reach its items too).
- The core never depends on sqlx, not even optionally: a consumer's domain layer depends on
  `queryweave` alone. `queryweave-api` never depends on sqlx either. `queryweave-codegen` and
  `queryweave-derive` depend on no runtime crate (only their tests do).
- Which integrations a struct uses is its flags, never a Cargo feature of a derive crate (features
  unify across the build, so one layer's choice would leak into every other). A runtime crate's
  features only add trait impls to its own types, or let generated code through: a derive cannot
  read a runtime crate's features (and a `#[cfg]` in its output tests the user's crate's), so code
  needing one goes inside a hidden `#[macro_export]` passthrough of that crate, defined once per state
  of the feature and reached through `__private` (`queryweave_sql::__private::if_diesel!`, which keeps
  the Diesel methods `Columns` generates only with the `diesel` feature, and `if_merge_patch!`, which
  keeps its merge-patch writes only with `queryweave-sql`'s `merge-patch`, whatever the derive's;
  `tests/ui/features/if_{diesel,merge_patch}.rs`).
- A derive used by several layers is one derive with roles (`Filters`), never one copy per layer.

### Generated code and paths

- Generated code names its runtime crate only through the path its mapping step resolved
  (`MappingPaths::runtime`): the one given by `#[queryweave(crate = path)]` (every derive accepts it,
  read with `attributes::crate_option`), or else the conventions' hook for the derive's layer
  (`C::core_crate()` / `api_crate()` / `sql_crate()`, absolute `::queryweave` etc. by default, so a
  local item cannot shadow them). Code generation takes that path as a value
  (`runtime: &syn::Path`, interpolated as `#runtime::…`), never a literal path or one assuming the
  call site's imports. In a check spanned to the user's code, respan the path (`utils::respanned`),
  or the error moves to the derive.
- Never make generated code name a third-party crate unconditionally.
- Every derive's output checks the runtime's version (`require_version`).
- The domain derives emit data tagged with the generated type it describes (`conditions()` →
  `Conditions<Self>`, `sort_keys()` → `SortKeys<Self>`, `changed_values()` → `FieldValues<Self>`, all
  `ModelData<M, _>`, built only by generated code through the hidden
  `queryweave::__private::model_data`; tests may use it, application code never), plus the domain
  fields that reach SQL (`PersistedFields::FIELD_NAMES`, on the generated filter-options / query
  models and on a `MergePatch` model) and, on the filter-options and `MergePatch` models, their
  value types (`FIELD_TYPES`, each `IntoValue::TYPE` of the field's raw value). SQL is built only
  in `queryweave-sql` and the code its derives generate.
- Domain field names become column names only through an entity deriving `Columns`
  (`#[domain_field(name)]` where they differ; honours `#[sqlx(rename)]` / `#[sea_orm(column_name)]` /
  `#[diesel(column_name)]`, which must agree, skips `#[sqlx(skip / flatten)]` / `#[sea_orm(ignore)]` /
  `#[diesel(embed / select_expression)]` and `#[columns(skip)]` (a column storing no domain field,
  e.g. of a SQL type queryweave cannot convert), rejects `rename_all`). An entity with
  `#[diesel(table_name = path)]` also gets `diesel_conditions` / `diesel_order_by` /
  `diesel_changeset`: one `match` arm per column naming `path::column` (the user's span), and one
  `where` predicate per column (`FilterColumn` / `OrderColumn` / `AssignColumn`) spanned to its field,
  so a column of an unsupported SQL type is reported there; and `diesel::Entity<'q, DB>` under the
  same bounds, through which the wrappers get `diesel_condition` (`AppendConditions`) and
  `diesel_apply_to` (`AppendToQuery`, on a boxed query), also inside `if_diesel!`. SQL
  for a model's data is written only through the entity's `Columns` methods (`push_conditions` /
  `push_order_by` / `push_assignments`; the derives also emit them, and `append_to_query`, as inherent
  methods, so callers import none of `queryweave-sql`'s traits). They run the column check
  themselves, so a missing column is reported at their caller, then call the query's unchecked
  `write_*` method (its target trait: `ConditionsTarget`, `OrderByTarget`, `AssignmentsTarget`,
  `PageTarget`), so generated code names no query builder. The query and the model are inferred from
  the arguments (the data's `ModelData<Model, _>` type, so data can't be paired with another model),
  and the call fails to build unless every field of `Model` has a column on `Entity`: a
  post-monomorphization check, so `cargo build`, not `cargo check`. The wrapper derives also emit
  `assert_columns!`, which reports it in `cargo check` too.

### Portable SQL

- `queryweave-sql` and its generated code stay database-agnostic: generated code is generic over the
  query's target traits, and their sqlx impls over `DB: queryweave_sql::BindValues` (Postgres,
  MySQL/MariaDB, SQLite), standard SQL only (no
  `RETURNING`, `ILIKE`, `::` casts or quoted identifiers), every value bound, never interpolated;
  `LIKE` patterns escape with `!` (`ESCAPE '!'`), not `\`. The sea-query backend builds sea-query
  expressions instead of text, the same comparisons, binds and `LIKE … ESCAPE '!'`; sea-query renders
  them per database, quoting identifiers itself; it binds a value whatever its column's type, so
  `sea_query::check_column_types` checks a SeaORM entity's column types (`(name, ColumnType,
nullable)`, handed over by the caller: no SeaORM dependency) against a model's `FIELD_TYPES` at
  run time, from a test or at startup. The Diesel backend builds Diesel's typed expressions:
  a `Value` converts to the column's Rust type through `ValueSqlType`, implemented per Diesel SQL type
  (lossless widenings only), and every function dispatches on the column's SQL type
  (`Comparison`, `ColumnValue`) so generated code never names or classifies one. A domain field
  whose value does not fit its column fails to build where `diesel_conditions` / `diesel_changeset`
  is called (`FIELD_TYPES` against each column's `FilterColumn::ACCEPTS` / `AssignColumn::TYPE`,
  from `ValueSqlType::ACCEPTS`, generated from the same list as the conversion; a nullable field needs a nullable column only to
  be written); what still does not fit at run time (a hand-written `PersistedFields`) is
  `diesel::ValueMismatch`, never a panic. Diesel binds the `LIKE` escape character
  (`ESCAPE $2`), still `!`.
- A new SQL helper gets a rendering test per database and runs in `queryweave-sql/tests/sqlite.rs`
  (and, where behaviour may differ, the Postgres / MySQL / MariaDB tests), for each backend
  (`tests/sea_query_sqlite.rs` for sea-query, `tests/diesel_sqlite.rs` for Diesel).
- Known semantic differences, documented rather than hidden: `LIKE` case sensitivity, unquoted
  column names (sqlx; sea-query quotes them, so they match case-sensitively on Postgres), out-of-range
  writes (MySQL clamps outside strict mode), no `u64`.

### Linking types across layers

- A derive never rebuilds another type's generated names from a naming pattern: it links to the
  path the user wrote. A `Filters` mirror names its mirrored model's generated types through
  `<Model as queryweave::Filterable>::Query` (etc.), builds its query with `Filterable::query` (a
  root's query and a mirror's query params are shaped differently), and converts into them by
  implementing the role traits (`QueryOf<Model>`, ...); the mirrored model's derive (root or mirror
  alike) turns those into `From` impls. A mirror cannot `impl From<Dto> for <Model as
Filterable>::Query` itself: coherence cannot rule out the projection being `Dto`, so it overlaps
  `impl<T> From<T> for T` (E0119). Projections never go in `impl` headers.
- Mirrors chain (e.g. a gRPC DTO mirroring an API DTO mirroring a domain model); every chain ends at
  a root model, `Filterable::Root`. A struct linking to itself (`Self`, or its own name without a
  module) is a macro error at the path; a cycle of two structs is rustc's E0275 (`overflow evaluating
<X as Filterable>::Root`) at both `mirrors` paths, which cannot be reworded.
- A field named differently on the linked type maps with `#[equivalent_field(name)]` (a `Filters`
  mirror and `ValidateNonNullable`; an error on a root model); the struct's own name stays what its
  users see (query parameters, validation error keys).
- When a derive relates a field to a field of another type (DTO ↔ domain model, domain ↔ entity), a
  missing or stale name must be a compile error at the field (or its `#[equivalent_field]` name),
  never a runtime string mismatch: emit `checks::assert_fields_exist` for those names, unless the
  generated code already names them with the user's span (a struct literal or enum variant, which
  rustc reports there).
- The derives never classify a field's type by its name: they only inspect its `Option` shape and
  leave the rest to the compiler, through `RawValue` (the raw value a field filters on and binds) and
  the filter-kind traits (`Ordered` for `lt` / `gt`, `Text` for `sw` / `c` / `ew`, `Boolean` for
  `bool`). Value objects are newtypes deriving `RawValue`
  (`#[derive(RawValue)] struct OrderNumber<T = String>(T);`); a new filterable raw type gets a
  `RawValue` (and filter-kind) impl in `queryweave`, not a macro change.

### Integrations

- A `Filters` mirror opts into third-party integrations per struct with flags,
  `#[filters(mirrors = path, serde, utoipa, validator)]` (`validator` needs `serde`; a flag on a root
  model is an error). A plain mirror names only the core; each flag only its own crate (`serde` /
  `utoipa` / `schemars` also the API runtime, for `SortDirectionDto` / `PageDto`). `schemars` (JSON
  schemas) and `garde` (validation needing no serializer) are the built-in alternatives to `utoipa`
  and `validator`.
- Each integration is an `Integration` impl in `queryweave_codegen::integrations` (public: its flag,
  what it requires, and its additions to each generated type: derives, attributes, `page_size` /
  `sort` attributes, items of any / of a sortable mirror, its sort direction and page types). The
  integrations a mirror may opt into are the conventions' (`Conventions::integrations()`,
  defaulting to `integrations::BUILT_IN`; a proc-macro crate adds or replaces some), in generated
  order. The `Filters` struct attributes are read against them (`TopLevelMeta<C>`), and the mapping
  step resolves the flags, checks each one's requirements and that they agree on the sort direction
  and page types. The mirror's code generation and the mapping step name no integration: a new one
  is a new impl, never a branch in `mirror.rs`.
- No integration relies on another's attributes (e.g. utoipa reading `#[serde(..)]`): each states
  the wire format itself. An unsortable mirror simply has no `sort` field.

### Merge patches

- "Absent, `null` or a value" is only `queryweave::Patch<T>`; `Option<T>` is a field that cannot be
  cleared. `Option<Option<T>>` is never supported: `MergePatchNullableFields` and
  `ValidateNonNullable` reject it at the field, naming `Patch<T>`, and `MergePatch` requires its patch
  model to derive `MergePatchNullableFields`. The derives apply and classify patch fields only
  through the traits `PatchField` / `PatchKind`.
- API patch DTO fields are `Patch<T>` with `#[serde(default)]` per field and
  `#[schema(value_type = Option<T>)]`. `Patch<T>`'s core features `serde`, `schemars` and
  `validator` (its `range` rule) are turned on by `queryweave-api`'s.
- `ValidateNonNullable` names no validation library: it always generates `rejected_nulls()`, and one
  flagged validation integration (`#[validate_non_nullable(mirrors = path, validator)]`, or `garde`)
  adds `validate_non_nullable()` in that library's errors. Its mirrors chain the same way as
  `Filters`: it asks the struct it mirrors through `NullableFields` (`is_nullable(field)` under that
  struct's names, and `Root`, the merge patch model the chain ends at; blanket-implemented for every
  `MergePatchNullableFields` model) and implements it itself, so a struct can mirror it in turn.

### The derive pipeline

- Every derive is a `queryweave_codegen::steps::Derive`: its associated types are its stages
  (`Input`, `TopLevelMeta`, `Parsed<'i>`, `MappingPaths`, `Model<'p>`, `Ir<'p>`), and `steps::expand`
  runs them in a fixed order, reporting the errors of independent stages together (struct
  attributes + parse, then mapping + analysis). The `Derive` impl in `pipeline/<derive>.rs` only
  delegates to the step modules, whose steps are free functions returning `syn::Result`
  (`parse::<derive>::parse`, `mapping::<derive>::generate_paths`, `analyze::<derive>::analyze`,
  `lower::<derive>::lower`, `code_gen::<derive>::code_gen`), each taking only what it needs.
- A skipped stage is an identity type: `()` for no model (every derive has struct attributes and
  mapping paths, since each reads `#[queryweave(crate = ..)]` and maps its runtime crate), and
  `&'p Parsed<'p>` as the IR of a derive generating code from its parsed input.
- Never add a crate-local step trait or wire a derive by hand in `lib.rs`.
- Macro failures are reported through `queryweave_codegen::errors` (spanned message + `help`), never
  `panic!`. Collect several errors in a step with `errors::Errors` (or `errors::both`), never as a
  `TokenStream`.

### Struct attributes

- The struct attributes a derive reads (`#[merge_patch(patch = ..)]`, `#[columns(model = ..)]`,
  `#[append_to_query(columns = .., filters = ..)]`, `#[append_conditions(columns = ..)]`,
  `#[filters(mirrors = ..)]`,
  `#[validate_non_nullable(mirrors = ..)]`, and `#[queryweave(crate = ..)]`, which every derive reads
  and registers) are its `Derive::TopLevelMeta`: `parse_top_level_meta/<derive>.rs` defines a
  `TopLevelMeta` struct (one field per attribute or option) implementing
  `steps::TopLevelMeta::from_attributes`, reading each with `attributes::path_option` (optional
  `#[name(key = path)]`), `required_path_option` (or `path_option` + `require_option`, to report
  several missing options of one attribute together) or `flag_option` (a bare `#[name(key)]`), each
  given every option of the attribute with its `OptionKind`. The option readers reject unknown,
  repeated and malformed options. Never hand-parse a struct attribute.
- Every link to another type is an option of the derive's own attribute, named after what it links
  to (`mirrors`, `patch`, `model`, `columns`, `filters`); never a bare `#[name(path)]` attribute.
- New attributes are registered in `#[proc_macro_derive(..., attributes(...))]`.

### Naming

- Every naming rule goes through `queryweave_codegen::conventions::Conventions`: the names a derive
  generates (types: `filters_names` → `FiltersNames`; members: `sort_variant`, `filter_field`) and
  the names it requires of a struct (its own, in `parse/`, or one a struct attribute links to, in
  `mapping/`: `check_model_name`, `check_merge_patch_name`, `check_wrapper_name`). Never build a
  generated name or check a struct name outside a `Conventions` impl.
- Type names are local to a layer; member names are shared, since a mirror names the mirrored
  model's sort variants and filter fields with its own `sort_variant` / `filter_field`.
  `DefaultConventions`: `OrderModel` → `OrdersQueryModel`, `OrderSortFieldModel`, ...; `OrderDto` →
  `OrdersQueryParams`, ...; so renaming a model renames them all.
- Pipelines are generic over the conventions (`pipeline::filters::Filters<C>`) and the step
  functions take them as a type parameter (`parse::<C>`, `analyze::<C>`, `generate_paths::<C>`);
  only a proc-macro crate picks one.
- Errors point at what the user wrote: the struct's name, or the path inside the attribute
  (`names::path_name` extracts the linked struct), never the call site when there is something
  better to underline.

## Tests

- **Every new derive, attribute or error path gets a trybuild `fail` case.**
- Unit-test each pipeline step directly (`parse`, `analyze`, `lower`, `code_gen`), with `insta`
  debug snapshots for parser output and code-generation snapshots (`.snap` files committed under
  `snapshots/`); `rstest` cases (give each case an explicit snapshot name), `pretty_assertions`.
- Behaviour of the generated code: `queryweave-derive/tests/<derive>.rs`.
- trybuild: `tests/ui/<derive>/{pass,fail}/*.rs`, the expected diagnostic committed as a sibling
  `.stderr` using `$DIR` paths, except where trybuild cannot normalize to them (diagnostics listing
  `and $N others`, or quoting another crate's source as `$WORKSPACE/…` or the standard library's as
  `$RUST/…`): keep trybuild's `TRYBUILD=overwrite` output for those.
- Conventions: `queryweave-custom-conventions`' tests check that every `Conventions` rule reaches the
  generated code; a new rule gets a case there.
- SQL: rendering per database plus real SQLite (in memory, fast suite) for each backend; real
  Postgres, MySQL 8.4 and MariaDB 11.4 through testcontainers in
  `queryweave-sql/tests/{postgres,mysql}.rs` (sqlx) and `tests/diesel_{postgres,mysql}.rs` (Diesel,
  run by `diesel-async`, whose pure Rust drivers need no libpq or mysqlclient). `queryweave-derive/tests/sea_orm.rs` runs the generated
  code against real SeaORM entities (`sea-orm` dev-dependency), `tests/diesel.rs` against a real
  Diesel entity on SQLite. The SQLite tests of sqlx and Diesel
  share one bundled `libsqlite3-sys` (pinned to sqlx's 0.30 line: one `links = "sqlite3"` per
  build).
- `queryweave/tests/no_sqlx.rs` guards that the core stays free of sqlx.

## Docs

For people first: `README.md` (what it is, for a newcomer) · `docs/guide.md` (every derive: what you
write and what you get) · `docs/integrations.md` (the flags, and writing your own) ·
`docs/conventions.md` (naming rules and runtime paths of your own) · `docs/architecture.md`
(internals and design reasons, for contributors). For agents: `docs/ai/architecture-reference.md`
(condensed invariants), mirroring `architecture.md`'s sections, and this file.

Write docs for a person meeting the project for the first time: what it is and why it exists in
plain language, the why next to the what (problem → idea → walkthrough → reference), before any
dense reference. No emojis. Keep `docs/ai/architecture-reference.md` in sync with
`docs/architecture.md`, and this file in sync with both.
