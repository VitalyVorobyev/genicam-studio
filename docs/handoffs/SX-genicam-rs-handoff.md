# Handoff: Shared XML Crate — genicam-rs Side

## Context

GenICam Studio (`genicam-studio` repo) and `genicam-rs` both need GenICam XML parsing. Currently:

- **genicam-rs** has `genapi-xml` (full XML parser) + `genapi-core` (runtime evaluator with NodeMap, SwissKnife, caching, invalidation). These are already **transport-free** — no Zenoh, no async, no GigE dependencies.
- **genicam-studio** has `genicam_xml_model` — a simpler streaming parser that produces `UiGraph` (flat node map for UI rendering). It lacks pValue resolution, SwissKnife evaluation, StructReg expansion, and dependency tracking.

The user wants both repos to share the same XML parsing crates. Studio needs offline XML browsing with full node resolution — no camera service required.

## Decision

**genapi-xml and genapi-core stay in genicam-rs.** Studio depends on them via git path:

```toml
# In genicam-studio/crates/genicam_xml_model/Cargo.toml
[dependencies]
genapi-xml = { path = "../../../genicam-rs/crates/genapi-xml" }
genapi-core = { path = "../../../genicam-rs/crates/genapi-core" }
```

No new repo. No crates.io publishing (yet). The crates are already standalone — this handoff asks for minimal changes to make them easier to consume externally.

## What We Need from genicam-rs

### 1. Stable public API for genapi-xml

**Current state:** `genapi-xml` exports `parse()`, `XmlModel`, `NodeDecl`, and related types. This is sufficient.

**Requested:**
- Ensure `XmlModel` and all `NodeDecl` variants derive `Clone` + `serde::Serialize` + `serde::Deserialize`. Studio needs to serialize parsed XML to JSON for the frontend.
- Ensure `AccessMode` derives the same.
- Document the public API with `///` doc comments on all public types and functions.

**Why serde?** Studio's UiGraph is a JSON contract between Rust and TypeScript. We need to convert `NodeDecl` → `UiNode` in Rust, and serde on the source types makes this straightforward. We won't serialize `XmlModel` directly to the frontend — we'll map it to our `UiGraph` — but serde derivations make the mapping code cleaner and enable debug serialization.

### 2. Stable public API for genapi-core

**Current state:** `genapi-core` exports `NodeMap`, `Node`, `RegisterIo`, `GenApiError`, `SkOutput`.

**Requested:**
- `NodeMap` needs a **read-only introspection API** (some of this exists):
  - `node_names() -> impl Iterator<Item = &str>` — list all nodes
  - `node(name: &str) -> Option<&Node>` — already exists
  - `dependents(name: &str) -> &[String]` — nodes invalidated when `name` changes
  - `categories() -> impl Iterator<Item = (&str, &[String])>` — category name → children
- `Node` enum variants should expose metadata without requiring `RegisterIo`:
  - Display name, description, tooltip, visibility, access mode, unit
  - For Integer/Float: min, max, inc (static from XML, not runtime-resolved)
  - For Enum: entry names and values
  - For SwissKnife/Converter: expression string, variable names
- Add `Node::kind_name() -> &'static str` (e.g., "Integer", "Float", "Enumeration")
- Add `Node::access_mode() -> Option<AccessMode>` accessor

**Why?** Studio needs to build a UI tree from the NodeMap without performing register I/O. The introspection API lets us construct `UiGraph` with full dependency information, computed constraints, and proper categorization.

### 3. No-IO evaluation mode for offline browsing

**Current state:** All `get_*` / `set_*` methods on `NodeMap` require `&impl RegisterIo`. For offline XML browsing (WASM, no camera), there's no IO.

**Requested:** A `MockRegisterIo` or `NullRegisterIo` that returns zeros for all reads:
```rust
pub struct NullIo;
impl RegisterIo for NullIo {
    fn read(&self, _addr: u64, len: usize) -> Result<Vec<u8>, GenApiError> {
        Ok(vec![0u8; len])
    }
    fn write(&self, _addr: u64, _data: &[u8]) -> Result<(), GenApiError> {
        Ok(())
    }
}
```

This allows Studio to call `nodemap.get_integer("Width", &NullIo)` to get default/zero values, and more importantly to evaluate SwissKnife expressions that depend only on XML-defined constants.

**Alternative:** If a built-in NullIo feels wrong, we can define it on the studio side. The trait just needs to be public.

### 4. WASM compatibility

**Current state:** Unknown whether genapi-xml + genapi-core compile to wasm32-unknown-unknown.

**Requested:** Ensure both crates compile under `wasm32-unknown-unknown`. This means:
- No `std::fs`, `std::net`, `std::process` usage
- No `tokio` dependency (genapi-core is sync — should be fine)
- `tracing` is wasm-compatible ✓
- `quick-xml` is wasm-compatible ✓

**Test:** `cargo build --target wasm32-unknown-unknown -p genapi-xml -p genapi-core`

If there are blockers, feature-gate them: `#[cfg(not(target_arch = "wasm32"))]`.

### 5. Keep `fetch_and_load_xml` optional

The `fetch_and_load_xml` function in genapi-xml uses async + closures for reading device memory. Studio doesn't need this (it gets raw XML from Zenoh). If it pulls in async dependencies or causes wasm issues, gate it behind a feature:

```toml
[features]
default = ["fetch"]
fetch = []  # Enables fetch_and_load_xml (async device memory reading)
```

## What Studio Will Do (no action needed from genicam-rs)

1. **Add genapi-xml + genapi-core as dependencies** via git path
2. **Write a mapping layer** in `genicam_xml_model`: `XmlModel` → `UiGraph`
   - Categories from `NodeDecl::Category` → `UiCategory`
   - Each `NodeDecl` variant → `UiNode` with appropriate `UiNodeKind`
   - pValue references → optional `dependencies` field on `UiNode`
   - SwissKnife expressions → stored for display (not evaluated in frontend)
3. **Keep existing `parse_genicam_xml()`** as a fallback (quick-xml streaming parser)
4. **Add `parse_full()`** that uses genapi-xml for richer output
5. **Build UiGraph with dependency edges** for the feature browser visualization (FB-03)
6. **WASM builds** will use genapi-xml for offline XML browsing with full node types

## Timeline

| Task | Owner | Duration | Depends on |
|------|-------|----------|------------|
| SX-01: API design + ADR | Joint | 1 week | — |
| SX-02a: Add serde derives to public types | genicam-rs | 2-3 days | SX-01 |
| SX-02b: Add introspection API to NodeMap/Node | genicam-rs | 1 week | SX-01 |
| SX-02c: WASM compatibility check + fixes | genicam-rs | 2-3 days | SX-01 |
| SX-02d: Doc comments on public API | genicam-rs | 2-3 days | SX-02b |
| SX-03: Integrate into studio | studio | 1-2 weeks | SX-02b |
| SX-04: Offline browsing in WASM | studio | 1 week | SX-02c, SX-03 |

Total: ~5-7 weeks with some parallelism.

## Bug: `--iface lo0` Doesn't Discover on Loopback

**Severity:** Blocks all loopback testing (fake camera + service on same machine).

**Root cause:** `genicam-service` calls `gige::discover_on_interface(timeout, "lo0")` which maps to `discover_impl(timeout, Some("lo0"), false)`. The `include_loopback=false` parameter at `tl-gige/src/gvcp.rs:205` causes the loopback interface to be skipped even when the user explicitly requests it via `--iface lo0`.

**File:** `crates/tl-gige/src/gvcp.rs` lines 201-206:
```rust
pub async fn discover_on_interface(
    timeout: Duration,
    interface: &str,
) -> Result<Vec<DeviceInfo>, GigeError> {
    discover_impl(timeout, Some(interface), false).await  // BUG: false should be true when iface is loopback
}
```

**Fix (option A — minimal):** Change `discover_on_interface` to detect loopback:
```rust
pub async fn discover_on_interface(
    timeout: Duration,
    interface: &str,
) -> Result<Vec<DeviceInfo>, GigeError> {
    let include_loopback = interface == "lo0" || interface == "lo";
    discover_impl(timeout, Some(interface), include_loopback).await
}
```

**Fix (option B — always include when filtered):** When a user explicitly names an interface, they want to use it regardless of type. Always pass `true`:
```rust
pub async fn discover_on_interface(
    timeout: Duration,
    interface: &str,
) -> Result<Vec<DeviceInfo>, GigeError> {
    discover_impl(timeout, Some(interface), true).await
}
```

Option B is cleaner — if someone says `--iface lo0`, they mean it.

**Verification:** After the fix:
```bash
arv-fake-gv-camera-0.8 -i 127.0.0.1 &
cargo run -p genicam-service -- --iface lo0 -vv
# Should log: "GVCP discovery ... interface_name=lo0 local=127.0.0.1 dest=127.255.255.255:3956"
# Should discover cam-000000000000
```

## Non-Goals

- **No breaking changes to genicam-rs internals.** We're asking for additive API surface only.
- **No runtime coupling.** Studio will not call `NodeMap::get_integer()` with a real `RegisterIo` — that's the service's job. Studio only needs metadata introspection.
- **No publishing to crates.io** yet. Git path dependency is sufficient for now.
- **No changes to genicam-service.** The service continues to work as-is.

## Verification

After genicam-rs changes:
1. `cargo test` in genicam-rs — all 12 integration tests still pass
2. `cargo build --target wasm32-unknown-unknown -p genapi-xml -p genapi-core` succeeds
3. `cargo doc -p genapi-xml -p genapi-core` produces clean docs
4. Studio can: `let model = genapi_xml::parse(&xml)?; let nodemap = NodeMap::try_from_xml(model)?;` — compiles and produces a valid NodeMap from any GenICam XML fixture

## Contact

This handoff is for the Claude instance operating on `genicam-rs`. Questions about Studio's UiGraph contract: see `crates/genicam_xml_model/src/model.rs` and `ui/genicam-studio-ui/src/xml_model/uigraph.ts`.

Questions about the Zenoh API contract: see `docs/zenoh-api.md` and `crates/genicam_zenoh_api/src/lib.rs`.
