# mamut-vizia

`vizia`-toolkit GUI track for `EPM1`, introduced by ADR 0002 Phase 1.

## Status

Phase 1, Step 2 — crate skeleton. `cargo run -p mamut-vizia` opens a
blank window with a placeholder label and exits. The PERFORM-screen
spike (`AppModel`, `state_bridge`, perform-rail, slot grid, macro
meters, oscilloscope, rotary knob) is the next step on this track.

## vizia pin

`vizia` is pinned by git rev, **not** by version range, per ADR 0002:

- repo: `https://github.com/vizia/vizia.git`
- rev: `56f4c964a2c0d9d256adc5e5ad166d4195516173` (vizia 0.4.0 main, 2026-05-09)

Do not bump this rev without:

1. Re-running the Phase 1 kill criteria from ADR 0002.
2. Updating both `Cargo.toml` and this README in the same change.
3. Running `tools/review/run-rust-subsystem-review.sh crates/mamut-vizia`
   to confirm the bump did not regress the spike.

If a breaking upstream change forces an unbudgeted rev bump during
Phase 3, the ADR 0002 off-ramp to `egui` applies; see
`docs/adrs/0003-gui-information-architecture.md`.

## Coexistence with `mamut-standalone`

`mamut-standalone` (the `egui` GUI) keeps shipping unchanged through
ADR 0002 Phases 1 through 3. The two toolkits coexist as separate
workspace binaries; there is no feature flag entanglement. The Phase 3
cutover removes `crates/mamut-standalone/src/gui/` and re-targets
`mamut-standalone` to launch `vizia` by default.

## Reading order

1. `../../docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md` —
   toolkit decision, phase plan, off-ramp, Phase 1 kill criteria.
2. `../../docs/adrs/0003-gui-information-architecture.md` — four-screen
   IA, cut rule.
3. `../../docs/ui/vizia-design-system.md` — palette, typography, knob
   geometry, panel chrome, screen wireframes.
4. `../mamut-runtime/src/session/state_source.rs` — read surface this
   crate consumes from the audio runtime.
