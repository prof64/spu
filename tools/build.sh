#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release --target wasm32-unknown-unknown --lib
bindgen_bin="${WASM_BINDGEN_BIN:-}"
if [[ -z "$bindgen_bin" ]]; then
  if command -v mise >/dev/null 2>&1; then
    bindgen_bin="$(mise where cargo:wasm-bindgen-cli@0.2.129 2>/dev/null || true)/bin/wasm-bindgen"
  fi
fi
if [[ ! -x "$bindgen_bin" ]]; then bindgen_bin="$(command -v wasm-bindgen)"; fi
"$bindgen_bin" --target web --out-dir web/pkg --out-name sprite_lab target/wasm32-unknown-unknown/release/sprite_lab.wasm
