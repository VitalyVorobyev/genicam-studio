---
description: Full architect → implementer → reviewer pipeline with automated retry and commit
argument-hint: <task-id>
---

# /implement $ARGUMENTS

Full architect → implementer → reviewer pipeline for GenICam Studio backlog task `$ARGUMENTS`.
Runs automatically without pausing unless Open Questions arise.

---

## Phase 1 — Architect (read-only planning)

Act as the GenICam Studio Architect, but do not invoke subagent. Do NOT write or modify any code in this phase.

**Reading order (always do this first):**
1. `docs/backlog.md` — find task `$ARGUMENTS` and its milestone context
2. `AGENTS.md` and `CLAUDE.md` — constraints and invariants
3. `docs/zenoh-api.md` — Zenoh key schema and payload types
4. `crates/genicam_zenoh_api/src/lib.rs` — shared payload structs
5. All files directly relevant to the task (read before referencing them)

**Produce a blueprint with these sections:**
- **Task Summary** — what it does, why it matters, which milestone
- **Affected Files** — exact list of files to create/modify, one-line description each
- **Implementation Steps (phased)** — numbered phases with file + line-level guidance
- **Unit Test Plan** — for every new pure function: name, file, test cases, expected test names
- **Zenoh / UiGraph Touch Points** — keys/payload types read or written, with direction
- **Invariants Checklist** — for each item below, state whether this task touches it:
  - Unknown node preservation (`UiNodeKind::Unknown`)
  - No XML parsing in UI
  - No logic in `apps/` shells
  - No `unwrap()`/`expect()` outside tests
  - `bun` (not npm) for JS operations
  - JSON contract stability (`nodes_by_name`, `categories`, `root_category`)
  - Zenoh API stability
- **Docs to Update** — which of `CHANGELOG.md` / `README.md` / `docs/zenoh-api.md` need updating; for CHANGELOG specify the exact line
- **Quality Gate Plan** — which quality gate commands are relevant for this task
- **Open Questions** — ambiguities that need user clarification before implementation begins

**Write `docs/handoffs/$ARGUMENTS.md`** with this exact structure:

```markdown
# $ARGUMENTS: {Task Title}

## Spec

**What:** [one sentence — what this task implements]
**Why:** [one sentence — why it matters / what it unblocks]

**Files touched:**
- `path/to/file.rs` — [one-line description of change]

**Key decisions:**
- [non-obvious design choice and rationale]

**Unit tests required:**
- `fn test_xyz` in `file.rs` — [what it tests]

**CHANGELOG entry:** `- $ARGUMENTS: {one-line description}`

## Implementation
_Pending_

## Review
_Pending_
```

**Show the `## Spec` section to the user.**

If Open Questions cannot be resolved from the codebase, **stop and ask the user** before proceeding. Otherwise continue automatically.

---

## Phase 2 — Implementer

Act as the GenICam Studio Implementer, but do not invoke subagent. Follow the Phase 1 blueprint exactly. Read every file you plan to modify before touching it.

**Repo boundaries (non-negotiable):**
| Boundary | Rule |
|----------|------|
| XML parsing | Only in `crates/genicam_xml_model/`. Never in UI or `apps/` |
| Zenoh payload types | Only in `crates/genicam_zenoh_api/` (no `zenoh` crate dep there) |
| App shells | `apps/genicam-studio-tauri/` is thin glue: IPC wiring + windowing only |
| UI | Consumes `UiGraph` JSON contract; never parses XML directly |
| JS/TS tooling | Always use `bun`, never `npm` |

**Code invariants:**
- No `unwrap()` or `expect()` outside `#[cfg(test)]` — use `?` or return `Result`
- Unknown XML nodes survive as `UiNodeKind::Unknown { tag }` with `RawNode` preserved
- Tauri async tasks use `tauri::async_runtime::spawn` (not `tokio::spawn`)
- Managed state shared with background tasks must be `Arc<T>`
- `provider` typed as `XmlModelProvider` interface (not a union type)
- No `window.__TAURI__` calls outside provider files

**Unit tests (required):** For every new pure function (sync, no I/O, no async), write `#[cfg(test)]` tests in the same file. Cover happy path + at least one edge case. Name: `test_{function}_{scenario}`.

**Implementation workflow:**
1. Implement phases in order from the blueprint (usually: Rust types → Tauri commands → UI hooks)
2. Run `cargo fmt` after any Rust changes
3. After all Rust changes: `cargo clippy --all-targets --all-features -- -D warnings` — fix all warnings
4. Run `cargo test` for each affected crate; fix failures
5. After UI changes: `cd ui/genicam-studio-ui && bun run build` — fix errors
6. Append one line to `CHANGELOG.md` under `## [Unreleased]` using the exact entry from the Spec
7. Update `README.md` only if a user-visible capability was added or changed
8. Update `## Implementation` section of `docs/handoffs/$ARGUMENTS.md`:

```markdown
## Implementation

**Files changed:**
- `path/to/file.rs` — [what changed]

**Tests added:**
- `fn test_xyz` in `file.rs` — [what it tests]
- (or "none — no new pure functions introduced")

**Deviations from spec:** [describe any departures, or "none"]

**CHANGELOG:** Added `- $ARGUMENTS: ...` under [Unreleased]
```

**Do NOT:** refactor outside blueprint scope · add docstrings to unchanged code · create one-use abstractions · use `npm` · add `#[allow(...)]` without an explanatory comment.

---

## Phase 3 — Review Loop (automated, max 3 retries)

Act as the GenICam Studio Reviewer, but do not invoke subagent. Run all quality gates and check all invariants. Do NOT write production code.

**Review checklist:**

**A. Repo Boundary Violations (Blocking)**
- XML parsing outside `crates/genicam_xml_model/` — grep for `quick_xml`, `roxmltree`, `xmltree` in `apps/` or `ui/`
- `zenoh` dependency added to `crates/genicam_zenoh_api/Cargo.toml`
- Non-trivial computation added to `apps/genicam-studio-tauri/` beyond IPC wiring
- UI code parsing XML strings directly

**B. Code Safety (Blocking)**
- Bare `unwrap()` or `expect()` outside test files in Rust
- `tokio::spawn` used instead of `tauri::async_runtime::spawn` in Tauri backend

**C. Unit Test Coverage (Important)**
- Every new pure function (sync, no I/O) has `#[cfg(test)]` tests covering happy path + edge case
- Test names follow `test_{function}_{scenario}`
- Flag every untested pure function by name

**D. Unknown Node Preservation (Blocking if parser files changed)**
- `UiNodeKind::Unknown` branch still exists if `crates/genicam_xml_model/src/` was modified

**E. JSON Contract Stability (Blocking if `model.rs` or `uigraph.ts` changed)**
- `nodes_by_name`, `categories`, `root_category` field names unchanged; no removals or renames

**F. Zenoh API Stability (Blocking if `genicam_zenoh_api` changed)**
- No field removals or renames; serde rename attributes unchanged; `docs/zenoh-api.md` updated if API changed

**G. TypeScript Patterns (Important)**
- `provider` typed as `XmlModelProvider`; no `window.__TAURI__` outside providers; only `bun`

**H. Documentation (Important)**
- `CHANGELOG.md` has a new entry under `[Unreleased]` for this task
- `README.md` updated if a user-visible capability was added
- `docs/zenoh-api.md` updated if any Zenoh key or payload changed

**I. Quality Gates (run all, report pass/fail):**
```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cd ui/genicam-studio-ui && bun run build   # only if UI files changed
```

Only report findings with confidence ≥ 80%. When uncertain, say so and explain what would clarify.

**Update `## Review` section of `docs/handoffs/$ARGUMENTS.md`:**
```markdown
## Review

**Gate results:**
| Gate | Result |
|------|--------|
| cargo fmt | PASS / FAIL |
| cargo clippy | PASS / FAIL |
| cargo test | PASS / FAIL |
| bun run build | PASS / FAIL / SKIP |

**Issues found:** Blocking: N · Important: N · Minor: N

**Unit test coverage:** [covered / missing for: fn_name, fn_name2]

**Verdict:** ✓ Ready to commit / ✗ Blocking issues remain
```

**If Blocking Issues found:** switch back to Implementer role, fix ONLY the listed issues, re-run quality gates, then re-review. Repeat up to 3 total attempts. If still blocking after 3 attempts, stop and report:
> "Could not resolve blocking issues after 3 attempts. Manual intervention needed."
> Show remaining blocking issues and stop. Do not commit.

---

## Phase 4 — Important Issues

If only Important/Minor issues remain (no Blocking), present them and ask:
> "Review complete. Important issues found: [list]. Fix before committing? (yes / no / list which ones)"

Proceed based on user answer.

---

## Phase 5 — Commit

When review is clean (zero Blocking Issues):

1. `git diff --name-only HEAD` to get changed files
2. Stage all changed source files plus:
   - `docs/handoffs/$ARGUMENTS.md`
   - `docs/backlog.md`
   - `CHANGELOG.md`
   - `README.md` (if modified)
3. Look up task title from `docs/backlog.md`
4. Commit with Co-Authored-By trailer:
   ```
   git commit -m "feat($ARGUMENTS): {task title}"
   ```
5. Update `docs/backlog.md`: change task status to `✓ done`, move its row to the bottom of the epic table (below other done tasks), add `~~strikethrough~~` to ID and task name columns.
6. Stage and commit the backlog update:
   ```
   git commit -m "chore: mark $ARGUMENTS done in backlog"
   ```

---

## Phase 6 — Summary

Print:
- What was implemented (one paragraph)
- Link to handoff: `docs/handoffs/$ARGUMENTS.md`
- Next suggested task: first `planned` P0 or P1 task in current milestone from `docs/backlog.md`
