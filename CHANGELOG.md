# Changelog

What changed in each release of queryweave, and how to upgrade. The five
published crates (`queryweave`, `queryweave-api`, `queryweave-sql`,
`queryweave-derive`, `queryweave-codegen`) share one version: use the same
version of all of them (the generated code checks it at compile time).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Before `1.0`, every minor version may break the API (as Cargo assumes).

## [0.2.0] - 2026-10-15

The persistence layer is no longer tied to sqlx: `queryweave-sql` writes a
model's data into sqlx, sea-query / SeaORM or Diesel queries, and checks a
domain field's type against its column.

### Upgrading from 0.1

- **Joining conditions**: `push_conditions` and the conditions wrapper take a
  `Junction` instead of a separator string: `" AND "` becomes
  `Junction::And`, `" OR "` becomes `Junction::Or`.
- **`AppendToQueryWithSeparator` is now `AppendConditions`**: rename the
  derive, its attribute (`#[append_to_query(columns = ..)]` on that wrapper
  becomes `#[append_conditions(columns = ..)]`) and the method
  (`wrapper.append_to_query(&mut query, separator)` becomes
  `wrapper.append_conditions(&mut query, Junction::And)`). `AppendToQuery`
  and its `#[append_to_query(columns = .., filters = ..)]` are unchanged. A
  proc-macro crate of your own calls `queryweave_codegen::append_conditions`
  instead of `append_to_query_with_separator`.
- **sqlx is a feature of `queryweave-sql`**, on by default. Nothing changes
  with the default features; with `default-features = false`, add
  `features = ["sqlx"]`.
- **`IntoValue` has a required `TYPE` constant** (the `Value` variant it
  converts into). It only matters if you implemented `IntoValue` yourself;
  the derives and the built-in types provide it.

### Changed

- `queryweave-sql` writes into any query implementing its target traits
  (`ConditionsTarget`, `OrderByTarget`, `AssignmentsTarget`, `PageTarget`)
  rather than only sqlx's `QueryBuilder`: `Columns::push_conditions`,
  `push_order_by`, `push_assignments`, `push_page` and
  `AppendToQuery::append_to_query` are generic over the query; a
  `QueryBuilder` is still passed as before.
- Conditions are joined by a typed `Junction` (`And` / `Or`), not a
  separator string.
- `AppendToQueryWithSeparator` is renamed `AppendConditions` (see above).
- sqlx is behind `queryweave-sql`'s `sqlx` feature (on by default).

### Added

- **sea-query and SeaORM** (`queryweave-sql` feature `sea-query`): sea-query's
  `SelectStatement`, `UpdateStatement` and `DeleteStatement` are targets, so
  the generated methods write into SeaORM queries through
  `QueryTrait::query`. `Columns` honours `#[sea_orm(column_name)]` and skips
  `#[sea_orm(ignore)]` fields. SeaORM itself needs Rust 1.94; queryweave
  stays on 1.88.
- **SeaORM column type check**:
  `queryweave_sql::sea_query::check_column_types` compares a SeaORM entity's
  column types (from `Column::iter()` and `ColumnTrait::def()`) with a
  model's value types, once, from a test or at startup, and reports the
  first field whose value the column does not take, or that may be `None` in
  a `NOT NULL` column.
- **Diesel** (`queryweave-sql` features `diesel`, plus `diesel-postgres`,
  `diesel-mysql`, `diesel-sqlite` for each backend's date-time types): the
  `queryweave_sql::diesel` module, and for an entity with
  `#[diesel(table_name = ..)]`, the generated `diesel_conditions`,
  `diesel_order_by` and `diesel_changeset`, plus `diesel_condition` /
  `diesel_apply_to` on the wrappers (applied to a boxed query).
  `#[diesel(column_name)]` is honoured, `#[diesel(embed)]` and
  `#[diesel(select_expression)]` fields are skipped.
- **Compile-time Diesel type check**: a domain field whose value does not fit
  its Diesel column's SQL type, or that may be `None` and is written to a
  `NOT NULL` column, fails `cargo build` where the Diesel methods are called,
  naming the field, the column and what it takes.
- `#[columns(skip)]` on an entity field: a column storing no domain field
  (e.g. of a SQL type queryweave cannot convert).
- `ValueKind`, `ValueType`, `Value::kind` and `PersistedFields::FIELD_TYPES`
  (filled in by the `Filters` and `MergePatch` derives; it defaults to empty
  for a hand-written `PersistedFields`, which is then not type-checked).
- Helpers for a query target of your own: `like_pattern`, `escape_like`,
  `LIKE_ESCAPE`, `page_offset`, `sql_junction`.

### Fixed

- `Columns` generated merge-patch writes according to `queryweave-derive`'s
  `merge-patch` feature rather than `queryweave-sql`'s, so a build with the
  derive's default features and `queryweave-sql` without `merge-patch` failed
  to compile. The writes now follow `queryweave-sql`'s feature.

## [0.1.0] - 2026-10-11

The first release: the `Filters`, `MergePatch`, `MergePatchNullableFields`,
`RawValue`, `ValidateNonNullable`, `AppendToQuery`,
`AppendToQueryWithSeparator` and `Columns` derives, their runtime crates, and
SQL written through sqlx for Postgres, MySQL/MariaDB and SQLite.

[0.2.0]: https://github.com/eduardmitrica/queryweave/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/eduardmitrica/queryweave/releases/tag/v0.1.0
