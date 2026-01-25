# GenICam Studio

Monorepo for GenICam XML tooling, UI, and desktop app.

## Development

See `AGENTS.md` for the AI working agreement (standards + Definition of Done).

### Prerequisites (macOS)

- Rust (stable): `rustup` installed
- Node.js (recommended: 20+)
- `wasm-pack` (for browser-mode parsing): `brew install wasm-pack` (or `cargo install wasm-pack`)
- Tauri CLI v2: `cargo install tauri-cli --version '^2'`

### 1) Rust (model + parser)

```sh
cargo test
```

### 2) UI in browser mode (Vite + WASM parser)

One-time setup:

```sh
cd ui/genicam-studio-ui
npm ci
npm run wasm:build
```

Run dev server:

```sh
npm run dev
```

In the browser, load the fixture file:
- `crates/genicam_xml_model/fixtures/minimal.xml`

You should see:
- root category name
- node count
- category count

### 3) Desktop app (Tauri v2)

One-time setup (needed for `beforeDevCommand`):

```sh
cd ui/genicam-studio-ui
npm ci
```

```sh
cargo install tauri-cli --version '^2'
cd apps/genicam-studio-tauri
cargo tauri dev
```

Notes:
- `apps/genicam-studio-tauri/src-tauri/tauri.conf.json` points Tauri to the Vite dev server (`http://localhost:5173`).
- The Tauri dev command will start the UI dev server via `beforeDevCommand`.
## Feature Browser UI

The Feature Browser renders the UiGraph produced by Rust/WASM and lets you:
- browse the category tree
- search by node name or display name
- inspect/edit features in offline mode
- view Raw XML and a model debug JSON snapshot

Run it with:

```sh
cd ui/genicam-studio-ui
npm run dev
```

Load `crates/genicam_xml_model/fixtures/minimal.xml` to see the full workflow.
