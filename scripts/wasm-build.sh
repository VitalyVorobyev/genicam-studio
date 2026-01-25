#!/usr/bin/env bash
set -euo pipefail

# Build the WASM adapter into the Vite-consumable location.
#
# Important: wasm-pack resolves --out-dir relative to the crate directory being built,
# not the current working directory. We therefore use a path that "escapes" from
# `crates/genicam_xml_model_wasm/` back to the repo root and into the UI source tree.
#
# Module boundary: parsing stays in `crates/genicam_xml_model`; this script only builds
# the thin `genicam_xml_model_wasm` adapter so the browser can call into Rust.
wasm-pack build crates/genicam_xml_model_wasm \
  --target web \
  --out-dir ../../ui/genicam-studio-ui/src/wasm/genicam_xml_model_wasm \
  --out-name genicam_xml_model_wasm
