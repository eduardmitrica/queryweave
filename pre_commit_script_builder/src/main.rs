//! Builds `pre-commit.sh` into a binary, to be used as the repository's git hook.

use std::process::{Command, exit};

fn main() {
    let pre_commit_path = if std::env::args().any(|a| a == "--env=dev") {
        "'../pre-commit.sh'"
    } else {
        "'./pre-commit.sh'"
    };

    let passed = Command::new("sh")
        .arg("-c")
        .arg(pre_commit_path)
        .status()
        .is_ok_and(|status| status.success());

    if !passed {
        println!("Pre-commit checks failed.");
        exit(1);
    }
}
