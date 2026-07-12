---
name: mamut-field-gfm-measurement-path
description: How to validate GFM/engine hot-path perf claims in mamut-sint-sw — no criterion/bench harness exists; use the deterministic render examples + cost model
metadata:
  type: reference
---

mamut-sint-sw has **no criterion/`[[bench]]` harness** anywhere in the workspace
(checked crates/*/Cargo.toml + benches/). So execution-optimizer verdicts on the
GFM/engine hot paths rest on a **cost model + the test suite**, not micro-benchmarks.

The documented measurement/evidence path for GFM work is the deterministic render
examples under `crates/mamut-engine/examples/gfm_engine_*_render.rs` — they render
fixed note patterns through `Engine::process_block` at `BLOCK_FRAMES = 256` and dump
PCM + `GfmDiagnostics`. The strike-injection evidence example is
`gfm_engine_note_strike_ab_render.rs` (8-note pattern, A/B on cathedral-bloom /
ember-vault / razor-thaw). CI's release-smoke builds all examples and runs
`core_output_safety_sweep`.

**How to apply:** When asked to prove a hot-path perf/allocation change here, drive a
render example (or a targeted `cargo test -p mamut-field`/`-p mamut-engine`) and compare
`GfmDiagnostics` / signatures for behavioral invariance; state cycle/alloc estimates as a
cost model. Don't promise a benchmark number the repo can't produce, and don't add a
bench crate without an explicit task (workspace-member additions are gated per AGENTS.md).
The measurement host is an i7-4600U (see user auto-memory), not a fast desktop.
</content>
