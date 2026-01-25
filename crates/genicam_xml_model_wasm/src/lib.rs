//! WASM adapter for GenICam XML parsing.
//!
//! Tooling choice: we keep parsing in `genicam_xml_model` (Rust/native), and expose a thin
//! `wasm-bindgen` wrapper here so the browser can reuse the same logic without duplicating
//! any XML handling in TypeScript.
//!
//! Module boundary: this crate should stay small and only translate between JS values and
//! the `UiGraph` contract. No parsing logic lives here.

use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_xml_to_uigraph(xml: String) -> JsValue {
    match genicam_xml_model::parse_genicam_xml(&xml) {
        Ok(graph) => {
            // HashMaps serialize to JS Maps by default; force plain objects so the UI can
            // treat UiGraph like a JSON contract without extra adapters.
            let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
            match graph.serialize(&serializer) {
                Ok(value) => value,
                Err(err) => throw_js(format!("Failed to serialize UiGraph: {err}")),
            }
        }
        Err(err) => throw_js(format!("XML parse failed: {err}")),
    }
}

#[wasm_bindgen]
pub fn version() -> String {
    let version = env!("CARGO_PKG_VERSION");
    match option_env!("GIT_HASH") {
        Some(hash) => format!("{version}+{hash}"),
        None => version.to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn throw_js(message: impl AsRef<str>) -> JsValue {
    // wasm-bindgen will raise a JS exception; this never returns in practice.
    wasm_bindgen::throw_str(message.as_ref());
}

#[cfg(not(target_arch = "wasm32"))]
fn throw_js(message: impl AsRef<str>) -> JsValue {
    // Non-wasm builds (tests) can't throw JS exceptions; return a descriptive value instead.
    JsValue::from_str(message.as_ref())
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }
}

// Note: Full wasm-level tests can be added with `wasm-pack test --headless --firefox`.
// We keep a tiny host-side smoke test to avoid heavy CI setup for now.
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::parse_xml_to_uigraph;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn parse_smoke_test() {
        let xml = r#"<RegisterDescription><Category Name=\"Root\"/></RegisterDescription>"#;
        let value = parse_xml_to_uigraph(xml.to_string());
        assert!(value.is_object());
    }
}
