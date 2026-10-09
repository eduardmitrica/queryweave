#!/bin/sh
# The checks to pass before committing: `scripts/check.sh fast`, the ones CI
# runs that need no Docker (formatting, lints, unit and doc tests, rustdoc,
# every queryweave-api feature combination and the filters-only build).
set -eu

# -------- safe color setup (no errors when TERM/TTY missing) --------
if [ -t 1 ] && [ -n "${TERM:-}" ] && command -v tput >/dev/null 2>&1 && tput colors >/dev/null 2>&1; then
  GREEN="$(tput setaf 2)"
  RED="$(tput setaf 1)"
  NC="$(tput sgr0)"
  BOLD="$(tput bold)"
else
  GREEN=""; RED=""; NC=""; BOLD=""
fi
# --------------------------------------------------------------------

printf "Running pre-commit checks...\n"

if ! sh "$(dirname "$0")/scripts/check.sh" fast; then
  printf "\n%sPre-commit: Issues detected by scripts/check.sh.%s\n" "$RED" "$NC"
  exit 1
fi

printf "\n%s%sSuccess: All pre-commit checks passed%s\n\n" "$GREEN" "$BOLD" "$NC"
