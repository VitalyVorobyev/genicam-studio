#!/usr/bin/env bash
set -euo pipefail

# Build the WASM adapter into the Vite-consumable location.
# This keeps parsing in Rust while the UI just consumes the UiGraph JSON.
wasm-pack build crates/genicam_xml_model_wasm \
  --target web \
  --out-dir ui/genicam-studio-ui/src/wasm/genicam_xml_model_wasm \
  --out-name genicam_xml_model_wasm
