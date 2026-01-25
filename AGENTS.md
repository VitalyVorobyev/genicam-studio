# AI Working Agreement (AGENTS.md)

This repo is early-stage: keep changes small, additive, and test-backed.

## Core principles
- Early-stage no-BC: no breaking changes to public Rust APIs, JSON contracts, or UI behavior unless explicitly requested.
- Preserve unknown nodes: never drop unrecognized XML; store as `UiNodeKind::Unknown { tag }` and keep `RawNode` populated.
- No logic duplication: parsing/normalization lives in Rust crates; UI renders the `UiGraph` contract; apps glue.
- Minimal, clear data structures: prefer plain structs/enums; keep `RawNode` lightweight (debug-friendly, not a full DOM).
- Open/closed + small modules: extend via new handlers/types; avoid large modules; extract focused units when files grow.
- Comprehensive comments (only where needed): document parsing rules, invariants, and edge cases; avoid redundant line-by-line commentary.

## Repo boundaries
- `crates/`: reusable Rust libraries (parsing, types, normalization). No Tauri/UI code.
- `ui/`: React UI. No XML parsing here; consume `UiGraph` only.
- `apps/`: product shells (Tauri). Thin glue (windowing, dialogs, command wiring); no duplicated parsing.

## Rust standards
- Error handling: no `unwrap()`/`expect()` outside tests; return `Result` with context (which element/field failed).
- Parsing: prefer streaming (`quick-xml::Reader`); avoid loading whole DOM; minimize allocations/copies.
- Unknowns: unhandled tags/fields must be preserved in `RawNode` and surfaced as `Unknown` nodes (not dropped).
- Public API: keep types stable; add fields backward-compatibly (`Option`, defaults, and careful serde changes).
- Quality gate: `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` must pass for changed crates.

## TS/React standards
- Contract-first: UI depends on the serialized `UiGraph` shape only (no GenICam tag-based logic in React).
- Components: keep them small; prefer pure components + thin containers.
- State: single source of truth; avoid duplicated/derived state stored in multiple places.
- Tauri integration: guard with `window.__TAURI__` and keep platform-specific code behind small helpers.

## Fixtures, snapshots, tests
- Add/extend fixtures under `crates/*/fixtures/` using small synthetic XML whenever possible.
- Every parser behavior change requires tests; update `expected_*.json` snapshots intentionally.
- Fixtures should include at least one unknown node to keep the preservation invariant covered.

## Definition of Done
- Code: compiles; no warnings; no duplicated logic; modules remain small/intentional.
- Tests: unit tests updated/added; snapshots updated; `cargo test` passes (crate + workspace when relevant).
- Tooling: `cargo fmt` and `cargo clippy` are clean; UI build remains healthy when contracts change.
- Behavior: unknown nodes preserved; `UiGraph` JSON remains stable (or changes are explicitly requested).
