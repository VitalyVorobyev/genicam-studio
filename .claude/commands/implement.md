---
description: Full architect → implementer → reviewer pipeline with automated retry and commit
argument-hint: <task-id>
---

# /implement $ARGUMENTS

Full architect → implementer → reviewer pipeline for a GenICam Studio backlog task.
Runs automatically without pausing for approval unless the architect raises Open Questions.

---

## Step 1 — Architect

Spawn the `genicam-architect` agent with task ID `$ARGUMENTS`.

The architect will:
- Look up the task in `docs/backlog.md`
- Read all relevant source files
- Produce a full implementation blueprint
- Create `docs/handoffs/$ARGUMENTS.md` with the `## Spec` section

**Show the Spec section from `docs/handoffs/$ARGUMENTS.md` to the user.**

If the architect listed Open Questions that cannot be resolved from the codebase, **stop and ask the user** before proceeding. Otherwise continue automatically.

---

## Step 2 — Implementer

Spawn the `genicam-implementer` agent with:
- Task ID: `$ARGUMENTS`
- Handoff file path: `docs/handoffs/$ARGUMENTS.md`
- The full blueprint from Step 1

The implementer will:
- Implement all blueprint phases
- Write `#[cfg(test)]` unit tests for every new pure function
- Update `CHANGELOG.md` under `[Unreleased]`
- Update `README.md` if a user-visible capability changed
- Update the `## Implementation` section of the handoff file
- Run all quality gates and report results

---

## Step 3 — Review Loop (automated, max 3 retries)

Spawn the `genicam-reviewer` agent with:
- Task ID: `$ARGUMENTS`
- Changed files (from implementer summary or `git diff --name-only`)

The reviewer will run all quality gates and write the `## Review` section of the handoff file.

**If Blocking Issues are found:**
1. Extract the blocking issues from the reviewer's output
2. Re-spawn `genicam-implementer` with:
   - The same task ID and handoff file
   - Explicit list of blocking issues to fix
   - Instruction: fix ONLY the listed issues, then re-run quality gates
3. Increment retry counter
4. Re-spawn `genicam-reviewer`
5. Repeat until no Blocking Issues or retry count reaches 3

**If 3 retries exhausted with Blocking Issues still present:**
- Report to the user: "Could not resolve blocking issues after 3 attempts. Manual intervention needed."
- Show the remaining blocking issues and stop. Do not commit.

---

## Step 4 — Important Issues

If only Important/Minor issues remain (no Blocking), present them to the user and ask:
> "Review complete. Important issues found: [list]. Fix before committing? (yes / no / list which ones)"

Proceed based on user answer.

---

## Step 5 — Commit

When review is clean (zero Blocking Issues):

1. Get changed files: `git diff --name-only HEAD`
2. Stage all changed source files plus:
   - `docs/handoffs/$ARGUMENTS.md`
   - `docs/backlog.md`
   - `CHANGELOG.md`
   - `README.md` (if modified)
3. Look up the task title from `docs/backlog.md`
4. Commit:
   ```
   git commit -m "feat($ARGUMENTS): {task title}"
   ```
   With Co-Authored-By trailer.
5. Update `docs/backlog.md`: change the task's status to `✓ done` and move its row to the bottom of the epic table (below other done tasks), adding `~~strikethrough~~` to the ID and task name columns.
6. Stage and commit the backlog update:
   ```
   git commit -m "chore: mark $ARGUMENTS done in backlog"
   ```

---

## Step 6 — Summary

Print:
- What was implemented (one paragraph)
- Link to handoff: `docs/handoffs/$ARGUMENTS.md`
- Next suggested task: the first `planned` P0 or P1 task in the current milestone from `docs/backlog.md`
