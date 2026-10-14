#!/bin/sh
# queryweave's checks, from the workspace root (`queryweave/`).
#
# Usage: scripts/check.sh [fast|ci|all]
#   fast  formatting, lints, unit and doc tests, documentation, every
#         `queryweave-api` feature combination, `queryweave-sql` with each
#         of its backends, the filters-only build and the sqlx check (the
#         default; no Docker, no UI tests)
#   ci    fast, plus the database tests (Postgres, MySQL, MariaDB in
#         containers: Docker must be running)
#   all   ci, plus the trybuild UI tests (each case is a full `cargo build`)
set -eu

mode="${1:-fast}"
case "$mode" in
fast | ci | all) ;;
*)
  echo "usage: $0 [fast|ci|all]" >&2
  exit 2
  ;;
esac

cd "$(dirname "$0")/.."

step() {
  printf '\n== %s\n' "$1"
}

step "format"
cargo fmt --all -- --check

step "lints"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "unit tests"
cargo nextest run --workspace --profile fast

step "doc tests"
cargo test --workspace --doc

step "documentation"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# The workspace enables every feature of `queryweave-api` (cargo unifies them),
# so check it alone under each feature combination: lints, tests, and the
# feature checks' UI cases (`tests/ui/features`).
for features in "" serde utoipa schemars validator serde,utoipa,schemars,validator; do
  step "queryweave-api with features [$features]"
  cargo clippy -p queryweave-api --all-targets --no-default-features --features "$features" -- -D warnings
  cargo nextest run -p queryweave-api --no-default-features --features "$features" --ignore-default-filter
done

# Likewise `queryweave-sql` alone, with each of its backends (`sqlx`,
# `sea-query`), both and neither: lints, tests (but the database ones) and the
# feature checks' UI cases.
for features in merge-patch merge-patch,sqlx merge-patch,sea-query merge-patch,sqlx,sea-query; do
  step "queryweave-sql with features [$features]"
  cargo clippy -p queryweave-sql --all-targets --no-default-features --features "$features" -- -D warnings
  cargo nextest run -p queryweave-sql --no-default-features --features "$features" --ignore-default-filter \
    -E 'not (binary(postgres) | binary(mysql))'
done

# Without the default `merge-patch` feature: the filters alone.
step "filters only (no merge-patch)"
cargo clippy -p queryweave -p queryweave-api -p queryweave-sql -p queryweave-codegen -p queryweave-derive \
  --lib --no-default-features -- -D warnings
cargo clippy -p queryweave-sql --lib --no-default-features --features sqlx,sea-query -- -D warnings

if [ "$mode" = ci ] || [ "$mode" = all ]; then
  step "database tests (Docker)"
  cargo nextest run -p queryweave-sql --test postgres --test mysql
fi

if [ "$mode" = all ]; then
  step "UI tests"
  cargo nextest run --workspace -E 'test(/ui_tests/)'
fi

printf '\nAll %s checks passed.\n' "$mode"
