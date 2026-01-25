# GenICam Studio

Monorepo for GenICam XML tooling, UI, and desktop app.

## Development

Rust (workspace tests):

```sh
cargo test
```

UI (Vite dev server):

```sh
cd ui/genicam-studio-ui
npm install
npm run dev
```

Tauri (desktop app):

```sh
cd apps/genicam-studio-tauri
cargo tauri dev
```
