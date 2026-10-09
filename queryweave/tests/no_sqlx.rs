//! queryweave never brings sqlx into a domain: the core (what a domain depends
//! on), the API runtime and the derives stay free of it, under every feature,
//! and only the SQL runtime depends on it. A user's domain may still choose
//! sqlx itself; this guards against queryweave doing it for them.

use std::path::PathBuf;
use std::process::Command;

/// The crates of the dependency graph of the queryweave crate `name`
/// (normal and build dependencies, all targets), with the given features.
fn dependencies(name: &str, features: &str) -> std::io::Result<Vec<String>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(name)
        .join("Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .args([
            "tree",
            "--edges",
            "normal,build",
            "--target",
            "all",
            "--prefix",
            "none",
        ])
        .args(["--format", "{p}", features, "--manifest-path"])
        .arg(&manifest)
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "cargo tree failed for {name}: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(String::from)
        .collect())
}

/// The sqlx crates (`sqlx`, `sqlx-core`, ...) among `crates`.
fn sqlx_crates(crates: &[String]) -> Vec<&String> {
    crates
        .iter()
        .filter(|name| *name == "sqlx" || name.starts_with("sqlx-"))
        .collect()
}

#[test]
fn test_only_the_sql_runtime_depends_on_sqlx() {
    for features in ["--all-features", "--no-default-features"] {
        for name in [
            "queryweave",
            "queryweave-api",
            "queryweave-codegen",
            "queryweave-derive",
        ] {
            assert_eq!(
                sqlx_crates(&dependencies(name, features).unwrap()),
                Vec::<&String>::new(),
                "{name} ({features}) depends on sqlx"
            );
        }
    }
}

// The check itself can see sqlx: the SQL runtime depends on it.
#[test]
fn test_the_check_finds_sqlx_where_it_is() {
    let dependencies = dependencies("queryweave-sql", "--all-features").unwrap();

    assert_ne!(sqlx_crates(&dependencies), Vec::<&String>::new());
}
