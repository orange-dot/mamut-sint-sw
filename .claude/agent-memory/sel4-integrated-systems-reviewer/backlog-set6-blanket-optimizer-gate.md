---
name: backlog-set6-blanket-optimizer-gate
description: SET6 states the execution-optimizer gate once as a blanket rule for SET6-4..10; per-item gate lists intentionally omit it — omission is not exemption
metadata:
  type: project
---

The SET6 tonewheel backlog (Shared Boundary Constraints) states **once** that every slice adding DSP to the per-frame organ render path — `SET6-4` through `SET6-10` — carries `sel4-rust-execution-optimizer` on top of its listed gates. The per-item "Review Gates" sections list only *other* slice-specific additions and deliberately do not repeat the optimizer.

**Why:** the gate found the original per-item listing (optimizer named on -4/-8 only) made the other four hot-path slices (-6 scanner, -7 preamp, -9 cabinet, -10 character) read as exempt from a gate `docs/review-gates.md` mandates for render-loop work. The fix was the blanket rule, not six repetitions.

**How to apply:** when any SET6-4..10 slice arrives for review, require the optimizer pass even though the item's own gate list does not name it; treat a slice that skipped it as gate-incomplete. Extends [[backlog-review-gate-convention]] (per-item lists name slice-specific reviewers only).
