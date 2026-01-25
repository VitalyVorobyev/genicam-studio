# GenICam Studio

Monorepo for GenICam XML tooling, UI, and desktop app.

## Development

See `AGENTS.md` for the AI working agreement (standards + Definition of Done).

Rust (workspace tests):

```sh
cargo test
```

UI (Vite dev server):

```sh
cd ui/genicam-studio-ui
npm install
# Browser mode needs WASM built once.
npm run wasm:build
npm run dev
```

Tauri (desktop app):

```sh
cd apps/genicam-studio-tauri
cargo tauri dev
```
