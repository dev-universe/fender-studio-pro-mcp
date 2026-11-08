#!/usr/bin/env bash
set -euo pipefail

check_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$check_root"

for check_tool in cargo node; do
  if ! command -v "$check_tool" >/dev/null 2>&1; then
    printf 'Required development tool is missing: %s\n' "$check_tool" >&2
    exit 1
  fi
done

cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
node --test tests/js/*.test.mjs
