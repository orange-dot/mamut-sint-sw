---
name: backlog-review-gate-convention
description: EPM1 backlog docs list only slice-specific review gates per item and defer the integrated docs-truth pass to the closing slice — looks like drift vs review-gates.md but is house style
metadata:
  type: project
---

EPM1 backlog docs (SET4, SET5) present each slice's **Review Gates** section as a
*subset* of the mandatory sequence in `docs/review-gates.md`, and this is
deliberate house style, not drift.

Observed pattern (verified SET4-3/4 and SET5-1..7):
- Per-item gates list only the slice-specific reviewers: `sel4-rust-systems-reviewer`
  (always), plus `sel4-rust-execution-optimizer` when a hot path is touched.
- `sel4-rust-single-file-reviewer` (review-gates.md step 1, "riskiest changed
  files") is **not** named per item, even though every v0.1 slice adds a new leaf
  module.
- `sel4-integrated-systems-reviewer` is named only on the **closing consolidation
  slice** (SET4-8, SET5-7), even though review-gates.md makes it mandatory whenever
  `docs/` changes — and every slice writes an evidence doc + updates
  `docs/README.md` in the same commit.

**Why:** `docs/review-gates.md`'s literal mandatory sequence is broader than what
the per-item lists name; the operative convention is "per-item lists name the
slice-specific additions; the whole-set docs-truth integrated pass is consolidated
at the closing slice." SET4 is the active, blessed model doc and does exactly this.

**How to apply:** When reviewing a new EPM1 backlog, do **not** flag the per-item
gate subset as a blocker or should-fix by itself — check it against SET4 first. It
is at most a nit (optionally add one sentence clarifying the per-item lists are
additive to review-gates.md's mandatory sequence). Related: [[evidence-class-honesty]],
[[drift-set4-range-enumeration]].

CI cross-check that recurs in these backlogs: release-smoke (`.github/workflows/ci.yml`)
runs `cargo build --release -p mamut-engine --examples` and runs only
`core_output_safety_sweep` — so **mamut-engine examples are built by CI; mamut-dsp
examples are not.** A backlog that puts offline primitives in `crates/mamut-dsp/examples/`
and says "not built by CI release-smoke" is telling the truth.
