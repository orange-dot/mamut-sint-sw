---
name: boundary-tonewheel-chain-position
description: SET6 organ bus joins render_frame before macro_state.rs:302; master DC blocker + safety limiter are at process.rs block scope, downstream of the join
metadata:
  type: project
---

ADR 0006 (SET6-0) pins the tonewheel organ bus join point: after the per-voice
loop (ends `macro_state.rs:300`) and before the final body/mid-side stage
(`macro_state.rs:302`). Verified at the SET6-0 gate (2026-07-14).

**Why:** the ADR claims "the existing final saturation, DC blocker, and safety
limiter keep governing the output." Only the final saturation (`soft_clip`,
`macro_state.rs:312-320`) lives inside `render_frame` (returns at :353). The
master DC blocker and safety limiter are applied one scope up, at block level:
`engine/process.rs:54` (`master_dc_blocker.process`) and `:59`
(`master_safety_limit`), after `render_frame()` returns at `:53`. All three are
genuinely downstream of a pre-:302 join, so the governance claim holds — but the
three stages are NOT co-located.

**How to apply:** at the SET6-4 gate (first real `render_frame` organ join), the
integrated review must confirm the join lands after the voice loop and before
:302, and that the `Disabled` path is skipped so the pre-slice render stays
bit-identical (constant-cost rule). Do not accept a claim that the DC blocker /
limiter sit "at :302" — they are block-scope in `process.rs`. See
[[boundary-runtime-control-queue-continuous]] for the sibling SET5/SET6 boundary
notes.
