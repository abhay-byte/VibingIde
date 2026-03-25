#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RELEASE_EXE="${REPO_ROOT}/target/release/vibingide"
DEBUG_EXE="${REPO_ROOT}/target/debug/vibingide"

if [[ -x "${RELEASE_EXE}" ]]; then
  cd "${REPO_ROOT}"
  exec "${RELEASE_EXE}" "$@"
fi

if [[ -x "${DEBUG_EXE}" ]]; then
  cd "${REPO_ROOT}"
  exec "${DEBUG_EXE}" "$@"
fi

cd "${REPO_ROOT}"
exec cargo run --release --manifest-path "${REPO_ROOT}/Cargo.toml" -- "$@"
