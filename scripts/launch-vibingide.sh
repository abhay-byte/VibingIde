#!/usr/bin/env bash
set -euo pipefail

SOURCE_PATH="${BASH_SOURCE[0]}"
while [[ -L "${SOURCE_PATH}" ]]; do
  SOURCE_DIR="$(cd "$(dirname "${SOURCE_PATH}")" && pwd)"
  SOURCE_PATH="$(readlink "${SOURCE_PATH}")"
  [[ "${SOURCE_PATH}" != /* ]] && SOURCE_PATH="${SOURCE_DIR}/${SOURCE_PATH}"
done

SCRIPT_DIR="$(cd "$(dirname "${SOURCE_PATH}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RELEASE_EXE="${REPO_ROOT}/target/release/vibingide"
DEBUG_EXE="${REPO_ROOT}/target/debug/vibingide"

# Default to full Rust backtraces for launcher-driven runs unless the user overrides it.
export RUST_BACKTRACE="${RUST_BACKTRACE:-full}"

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
