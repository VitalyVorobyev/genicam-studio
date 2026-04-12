# /quality-gate

Run all GenICam Studio quality checks: fmt, clippy, test, bun build.

---

## Steps

Run each check sequentially. Report pass/fail after each step. Stop and report immediately on first failure — do not skip ahead.

### 1 — Rust Format Check
```bash
cargo fmt --check
```
- PASS: no output, exit 0
- FAIL: shows files with formatting issues → run `cargo fmt` to fix, then re-run check

### 2 — Clippy Lint
```bash
cargo clippy --all-targets --all-features -- -D warnings
```
- PASS: no warnings or errors
- FAIL: show all warnings/errors with file + line; each warning is treated as an error

### 3 — Rust Tests
```bash
cargo test
```
- PASS: all tests pass
- FAIL: show failing test names and error output

### 4 — UI Build
```bash
cd ui/genicam-studio-ui && bun run build
```
- PASS: build completes with no errors
- FAIL: show TypeScript/bundler errors

---

## Report Format

```
1. cargo fmt --check      ✓ PASS  /  ✗ FAIL
2. cargo clippy           ✓ PASS  /  ✗ FAIL  (N warnings treated as errors)
3. cargo test             ✓ PASS  /  ✗ FAIL  (N test failures)
4. bun run build          ✓ PASS  /  ✗ FAIL
```

If all pass: "All quality gates passed."
If any fail: list actionable fixes for each failure.
