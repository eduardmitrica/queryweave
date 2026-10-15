# queryweave architecture reference (for AI agents)

> A condensed, invariant summary of queryweave for agents working on it. It
> follows the sections of [../architecture.md](../architecture.md), which
> explains the reasons; usage is in [../guide.md](../guide.md),
> [../integrations.md](../integrations.md) and
> [../conventions.md](../conventions.md). When this file and the code
> disagree, the code is right: fix this file.

## Crates

| Crate                           | Kind       | Holds                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `queryweave`                    | lib        | Core runtime, named by code generated for root models: `Filterable`, role traits (`QueryOf`, `FilterOptionsOf`, `SortFieldOf`, `SortDetailsOf`), `IntoRootQuery`, `SortDirectionModel`, `ChangeSet`, `ChangedFields`, `MergePatchNullableFields`, `NullableFields`, `RawValue` + `Ordered` / `Text` / `Boolean`, `FilterOperator`, `Value` / `IntoValue` / `FilterCondition` / `SortKey` / `FieldValue`, `PersistedFields`, `ModelData` / `Conditions` / `SortKeys` / `FieldValues`, `__private::model_data` (generated code only). No sqlx, no serializer, no SQL text.                                                                                                                                                                                                                                                                                                                                        |
| `queryweave-api`                | lib        | API runtime: `PageDto<T>`, `SortDirectionDto` (converts both ways with `SortDirectionModel`), `features::{Serde, Utoipa, Schemars}`. Features `serde`, `utoipa`, `schemars`, none by default. Re-exports the core. Never sqlx.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `queryweave-sql`                | lib        | SQL runtime: `Columns` (+ `push_conditions`, `push_order_by`, `push_assignments`), the target traits (`ConditionsTarget`, `OrderByTarget`, `AssignmentsTarget`, `PageTarget`; sqlx's `QueryBuilder` and sea-query's statements implement them), `AppendToQuery`, `AppendConditions`, `assert_columns!`, `Junction`, `push_page`, `sql_operator`, `sql_direction`, `sql_junction`, `like_pattern` / `escape_like` / `LIKE_ESCAPE` / `page_offset` (for other targets), `BindValues` (feature `sqlx`), `__private` (generated code only). `diesel` module (feature `diesel`: typed functions over Diesel columns). `sea_query` module (feature `sea-query`: `check_column_types`, a runtime check of a SeaORM entity's column types). Features `sqlx` (backend, default), `sea-query` (backend), `diesel` (+ `diesel-postgres` / `-mysql` / `-sqlite`), `merge-patch`. Re-exports the core. No driver dependency. |
| `queryweave-codegen`            | lib        | Code generation: `filters::<C>`, `merge_patch::<C>`, `merge_patch_nullable_fields::<C>`, `validate_non_nullable::<C>`, `raw_value::<C>`, `columns::<C>`, `append_to_query::<C>`, `append_conditions::<C>` (`TokenStream -> TokenStream`). Public modules: `conventions`, `integrations`, `errors`, `names`. Private: `steps`, `attributes`, `checks`, `utils`, the step modules.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `queryweave-derive`             | proc-macro | Every derive under `DefaultConventions`, one line each. Exports only derives. Holds behavior tests and trybuild UI suites.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `queryweave-custom-conventions` | proc-macro | Test fixture: other conventions and an integration (`wire`). `publish = false`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |

Versioning: `0.2.0`, all five published crates in lockstep (`CHANGELOG.md`). License `MIT OR Apache-2.0`.
MSRV 1.88 (every feature; SeaORM, a dev-dependency, needs 1.94). A dependency between queryweave crates has `version` next to `path`.

## Invariants

### Crate boundaries

- A proc-macro crate exports only derives; runtime items live in a runtime crate.
- Each derive's generated code names exactly one runtime crate, the one of its layer.
- `queryweave` never depends on sqlx or a serializer; `queryweave-codegen` and
  `queryweave-derive` depend on no runtime crate (only their tests do).
- Shared plumbing goes in `queryweave-codegen`'s plumbing modules, never duplicated.
  A derive used by several layers is one derive with roles (`Filters`), never one per layer.

### Linking layers

- A derive links to the path the user wrote; it never rebuilds another type's
  generated names from a naming pattern.
- A `Filters` mirror names the mirrored struct's types through
  `<M as Filterable>::Query` (etc.), builds its query with `Filterable::query`,
  and converts by implementing the role traits, which the mirrored struct's
  derive turns into blanket `From` impls (not `impl From<Dto> for <M as
Filterable>::Query`: coherence rejects it, overlapping `impl<T> From<T> for T`).
- Mirrors chain; every chain ends at a root (`Filterable::Root`). A cycle is
  rustc `E0275` at both `mirrors` paths (the projection is spanned to them); a
  self-link (`Self`, own name, `self::` + own name) is a macro error
  (`attributes::reject_self_link`).
- Merge patches use `Patch<T>` (absent / `null` / value) for a field that may
  be cleared and `Option<T>` for one that may not; `Option<Option<T>>` is
  rejected by shape in `MergePatchNullableFields` and `ValidateNonNullable`
  (`attributes::reject_option_of_option`), with an error naming `Patch<T>`.
  The derives apply and classify fields only through `PatchField<F>` and
  `PatchKind`; `MergePatch` requires its patch model to derive
  `MergePatchNullableFields`. Non-patch fields of a patch model are
  `#[merge_patch(skip)]` (`attributes::is_skipped`).
- `ValidateNonNullable` names no validation library: it always generates
  `rejected_nulls()`; one flagged validation integration (`validator`,
  `garde`: `Integration::supports_validate_non_nullable` /
  `validate_non_nullable`) adds `validate_non_nullable()`. Default runtime:
  the core. `Patch<T>` has optional core features `serde`, `schemars`,
  `validator` (range), turned on by `queryweave-api`'s.
- `ValidateNonNullable` mirrors chain through `NullableFields` (`Root`,
  `is_nullable(field)` under the struct's own names); blanket impl over
  `MergePatchNullableFields`.
- A field related to another type's field (mirror ↔ mirrored, entity ↔ model)
  is checked at compile time at the field (`checks::assert_fields_exist`, or a
  user-spanned struct literal / variant), never as a runtime string.

### Persistence

- Domain field names become column names only through an entity deriving
  `Columns` (`#[domain_field(name)]` where they differ; `#[sqlx(rename)]` /
  `#[sea_orm(column_name)]` / `#[diesel(column_name)]`, which must agree;
  `#[sqlx(skip / flatten)]` / `#[sea_orm(ignore)]` /
  `#[diesel(embed / select_expression)]` / `#[columns(skip)]` are no columns;
  `rename_all` is rejected). `#[diesel(table_name)]` adds `diesel_conditions` /
  `diesel_order_by` / `diesel_changeset` (per-column `match`, per-column bounds
  spanned to the field), and `diesel::Entity<'q, DB>` (its `Table`, `conditions`,
  `order_by`, under the same bounds), inside `__private::if_diesel!`. Through
  `Entity`, the wrappers get `diesel_condition` (`AppendConditions`) and
  `diesel_apply_to` (query: `diesel::apply` on a `diesel::BoxedQuery`), also
  inside `if_diesel!`; no guard closure (the caller `filter`s the query).
- SQL for a model's data is written only through the entity's `push_*`
  methods, taking `ModelData<Model, _>` (data cannot be paired with another
  model); they check column coverage at build time (post-monomorphization),
  then call the query's unchecked `write_*` (its target trait).
  The wrapper derives also emit `assert_columns!` (reported by `cargo check`).
- SQL is database-agnostic: generic over the query's target traits (sqlx's
  impls over `DB: BindValues`; sea-query's build expressions, rendered and
  quoted per database by sea-query), standard SQL, every
  value bound, `LIKE ... ESCAPE '!'`. A new SQL helper gets a rendering test per
  database and runs in `queryweave-sql/tests/sqlite.rs` (sea-query:
  `tests/sea_query_sqlite.rs`).

### Field types

- The derives inspect only a field's `Option` shape; everything else goes
  through `RawValue` and the filter-kind traits. A new raw type is an impl in
  `queryweave`, not a macro change.

### Integrations

- A mirror's integrations are flags of `#[filters(...)]`, never Cargo features
  of a derive crate. Runtime-crate features only add trait impls to that
  crate's types, or let generated code through a per-feature passthrough
  macro of that crate (`__private::if_diesel!`), since a derive cannot read
  them.
- Each integration is an `integrations::Integration` impl; the available ones
  are `Conventions::integrations()` (default `BUILT_IN`: `serde`, `utoipa`,
  `validator`, `schemars`, `garde`), whose order is the generated order.
  `mirror.rs` and `mapping/filters.rs` name no integration; a new integration
  is a new impl, never a branch.
- Generated code never names a third-party crate unconditionally. No
  integration reads another's attributes; each states the wire format itself.
  An unsortable mirror has no `sort` field.
- The mapping step checks requirements (`validator` needs `serde`) and that
  integrations providing a sort direction or page type agree.
- An integration naming `queryweave-api` types emits a `features::*` check,
  spanned to its flag, emitted first.

### Pipeline

- Every derive is a `steps::Derive` in `pipeline/<derive>.rs`: associated types
  `Input`, `TopLevelMeta`, `Parsed<'i>`, `MappingPaths`, `Model<'p>`, `Ir<'p>`;
  methods only delegate to step functions (`parse::<d>::parse`,
  `mapping::<d>::generate_paths`, `analyze::<d>::analyze`, `lower::<d>::lower`,
  `code_gen::<d>::code_gen`), each a free function returning `syn::Result`.
  `steps::expand` runs them; independent stages' errors are reported together.
  Never a crate-local step trait or a hand-wired derive.
- Skipped stage: `()` (no model), or `&'p Parsed<'p>` as the IR.
- Struct attributes: `parse_top_level_meta/<d>.rs`, read only with
  `attributes::{path_option, required_path_option, require_option,
flag_option}`; every link is an option of the derive's own attribute
  (`#[filters(mirrors = ..)]`, `#[merge_patch(patch = ..)]`,
  `#[columns(model = ..)]`, `#[append_to_query(columns = .., filters = ..)]`,
  `#[append_conditions(columns = ..)]`,
  `#[validate_non_nullable(mirrors = ..)]`, `#[queryweave(crate = ..)]` on every
  derive). Never hand-parse an attribute.
- Errors through `errors` (spanned message + `help`), never `panic!`; several
  in a step with `errors::Errors` / `errors::both`, never as a `TokenStream`.
  Errors point at what the user wrote.
- Generated code names its runtime crate only via `MappingPaths::runtime`
  (`#[queryweave(crate = ..)]` or `C::core_crate()` / `api_crate()` /
  `sql_crate()`, absolute by default); never a path assuming the call site's
  imports. Respan it (`utils::respanned`) in a check spanned to user code.

### Features and versions

- `queryweave-sql`'s `sqlx` (default) and `sea-query` are its backends
  (`backend/sqlx.rs`, `backend/sea_query.rs`): only target-trait impls, whose
  `on_unimplemented` names the features; checked with each, both and neither
  (`check.sh`, `tests/ui/features/`, failing cases only without a backend).
  `sea-query` is also the public `queryweave_sql::sea_query` module:
  `check_column_types::<Entity, Model>(columns)`, a runtime check of a
  model's `FIELD_TYPES` against `(column name, sea_query::ColumnType,
nullable)` (SeaORM's `Column::iter()` / `def()`, passed by the caller: no
  SeaORM dependency); `ColumnTypeMismatch` names field, column and types.
- `diesel` (+ `diesel-postgres` / `-mysql` / `-sqlite`, mirroring Diesel's
  backend features for date-time SQL types) is the public
  `queryweave_sql::diesel` module (`backend/diesel.rs`): `condition`, `join`,
  `ordering`, `page`, `assignment`, `apply` over typed Diesel columns, dispatching on the
  column's SQL type (`ValueSqlType`, `Comparison`, `ColumnValue`). A domain
  field whose value does not fit its column fails to build (post-mono, in
  `diesel_conditions` / `diesel_changeset`): `PersistedFields::FIELD_TYPES`
  (from `IntoValue::TYPE`, filled by `Filters` / `MergePatch`) against
  `FilterColumn::ACCEPTS` / `AssignColumn::TYPE` (`ValueSqlType::ACCEPTS`,
  nullability; consts of the existing column bounds); a nullable field
  needs a nullable column only to be written. Hand-written models (no types)
  fall back to the runtime `ValueMismatch`. No target trait (Diesel builders are by
  value).
- `merge-patch` (default) in every queryweave crate gates the merge patch
  derives and runtime; `Columns` always emits `push_assignments` /
  `diesel_changeset`, inside `__private::if_merge_patch!` (the runtime's
  feature decides, not the derive's). Filters-only builds are checked by
  `scripts/check.sh`.
- Every derive's output ends with `require_version(MAJOR, MINOR)` in its
  runtime (`steps::expand`, `Derive::runtime`); pre-1.0 minors must match,
  post-1.0 runtime minor >= generator minor.
- Generated code never names sqlx: it is generic over the query's target
  traits (`queryweave-sql` still re-exports `QueryBuilder` and `Separated` for
  callers).

### Conventions

- Every naming rule goes through `conventions::Conventions` (`filters_names`,
  `filter_field`, `sort_variant`, `check_model_name`, `check_merge_patch_name`,
  `check_wrapper_name`, the crate paths, `integrations`). Pipelines and step
  functions are generic over `C`; only a proc-macro crate picks one.
- Type names are local to a layer; member names (`filter_field`,
  `sort_variant`) must agree across a chain.
- `#[filters(query | query_params | filter_options | sort_field |
sort_details | page = Name)]` renames one generated type of a struct
  (`parse_top_level_meta::filters::Renames`, applied in `mapping`); a rename
  of a type the role does not generate, or two generated names (or one and
  the struct) colliding, is an error at the name.

## Testing

- Unit tests per step, `insta` snapshots of generated code.
- Behavior: `queryweave-derive/tests/<derive>.rs`.
- trybuild: `tests/ui/<derive>/{pass,fail}/*.rs` with `.stderr` using `$DIR`
  paths (except files trybuild cannot normalize: `and $N others`,
  `$WORKSPACE/...`). Every new derive, attribute or error path gets a `fail` case.
  Regenerate with `TRYBUILD=overwrite`; UI suites run serialized and outside the
  `fast` / `ci` nextest profiles.
- `queryweave/tests/no_sqlx.rs`: only `queryweave-sql` depends on sqlx (`cargo tree`,
  all features and none).
- `queryweave-api` alone under each feature combination (unified away in a
  workspace build).
- Databases: `queryweave-sql/tests/sqlite.rs`, `tests/sea_query_sqlite.rs` and
  `tests/diesel_sqlite.rs`
  (in memory, in `fast`); `queryweave-derive/tests/sea_orm.rs` (real SeaORM
  entities);
  `tests/postgres.rs` and `tests/mysql.rs` (MySQL and MariaDB), and
  `tests/diesel_{postgres,mysql}.rs` (the Diesel backend, through
  `diesel-async`), start testcontainers, so need Docker and are left out of
  the `fast` profile.
- `queryweave/` is a Cargo workspace of its own. `scripts/check.sh fast`
  (no Docker: fmt, clippy, unit, doc, rustdoc, feature combinations,
  filters-only), `ci` (+ database tests), `all` (+ UI tests). The GitHub
  workflow runs `ci` and the UI tests.
- Strict lints: `unsafe_code = "forbid"`, `missing_docs`, pedantic clippy with
  `-D warnings`; rustdoc with 0 warnings.
