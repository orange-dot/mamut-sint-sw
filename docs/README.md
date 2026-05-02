# Docs

This directory is the local `EPM1` documentation entrypoint for the
`Mamut EPM` software line.

Ownership split:

- this repo `mamut-sint-sw` is the canonical implementation repo for `EPM1`
- sibling repo `mamut-sint-hw` is the canonical implementation repo for `EPM2`
- shared `Mamut EPM` identity framing still remains anchored across the broader
  umbrella docs corpus
- shared Linux audio platform planning is now anchored in the separate
  `mamut-platform` docs track

Start here:

- `mamut-epm-program-map.md` - shared program map for `Mamut EPM`, `EPM1`, and `EPM2`
- `EPM1_TRANSPORT_FREEZE.md` - local freeze/resume note for the last direct `EPM1` transport slice
- `EPM1_SPRINT_6_PC4_PERFORMANCE_RIG.md` - Sprint 6 backlog, acceptance bar, and live-rig contract
- `EPM1_SPRINT_6A_HOST_UNDERRUN_STABILITY.md` - next work item for real-host underrun diagnosis and performance stabilization
- `EPM1_PC4_LIVE_PROFILE.md` - locked Sprint 6 `PC4` control map, live slot policy, and performance-window truth model
- `EPM1_FIRST_PERFORMANCE_PLAYBOOK.md` - shortest trustworthy path for the first real `PC4` performance session
- `dsp/primitives-math.md` - formula companion for the low-level Rust DSP primitives
- `dsp/render-path-math.md` - frame-level math for voice allocation, mixing, filtering, and final render stages
- `dsp/control-identity-math.md` - macro, identity, and direct-parameter resolution math
- `live-sessions/` - lab evidence from real hardware runs; exact device names,
  ALSA selectors, and raw MIDI snippets are preserved as evidence, not portable
  defaults
- `live-sessions/2026-04-29-pc4-ag03-headless.md` - first documented real `PC4 -> mioXM DIN 1 -> EPM1 -> AG03` headless hardware session
- `live-sessions/2026-04-29-pc4-ag03-headless-performance-mode.md` - follow-up PC4 Performance mode run with startup entry events and stuck-held-note finding
- `live-sessions/2026-04-29-pc4-ag03-clean-idle-noteoff-fix.md` - clean-idle Global MIDI follow-up and repeated-pitch note-off engine fix
- `live-sessions/2026-04-29-pc4-ag03-noteoff-fix-validated.md` - hardware validation that the repeated-pitch note-off fix clears the stuck-note failure
- `live-sessions/2026-04-29-pc4-ag03-full-controller-map.md` - full one-way `PC4` sustain/K/S/SW/wheels/aftertouch hardware map validation into Mamut
- `live-sessions/2026-04-29-pc4-ag03-windowed-gui-redesign-live-take.md` - windowed GUI redesign live take, layout regression/fix, and real PC4 control evidence
- `live-sessions/2026-04-30-direct-output-capture-analysis.md` - first direct-output f32 WAV analysis, including valid recorder evidence and the DC-offset follow-up
- `live-sessions/2026-04-30-direct-output-dc-blocker-validation.md` - direct-output f32 WAV validation after the master DC blocker, with the remaining headroom and denormal follow-ups
- `EXTERNAL_DSP_REFERENCES.md` - local notes on external DSP references such as
  `AudioNoise` and `dspc`
- `dsp/next-generation-dsp-research-tracks.md` - index for non-contract
  next-generation DSP research directions
- `dsp/implementation-language-strategy.md` - Rust plus narrow C strategy for
  next-generation DSP implementation
- `dsp/dsp-core-polish-v0.1-output-safety-evidence.md` - core output safety
  polish evidence for headroom limiter and denormal flush
- `dsp/dsp-core-polish-v0.2-output-safety-sweep-evidence.md` - output safety
  telemetry and factory sweep evidence
- `dsp/material-core-idea.md` - exploratory material-memory stage idea
- `dsp/gravitational-phase-lattice-idea.md` - `GFM` radical alternate voice
  engine idea
- `dsp/gfm-v0.1-baseline.md` - first accepted audible `mamut-field` evidence
  baseline
- `dsp/gfm-v0.2-gesture-evidence.md` - attack-hold-release `mamut-field`
  gesture evidence
- `dsp/gfm-v0.3-gesture-variants-evidence.md` - combined short-strike,
  slow-press, and repeated-strike `mamut-field` evidence
- `dsp/gfm-v0.4-recovery-performance-evidence.md` - recovery-safe offline
  `mamut-field` performance evidence
- `dsp/gfm-v0.5-performance-contract.md` - public `mamut-field`
  performance program and gesture contract
- `dsp/gfm-v0.6-engine-dry-run-evidence.md` - first one-way
  `mamut-engine -> mamut-field` dry-run evidence
- `dsp/gfm-v0.7-engine-block-evidence.md` - block-rendering
  `mamut-engine -> mamut-field` evidence
- `dsp/gfm-v0.8-patch-selection-evidence.md` - offline factory patch
  identity-to-GFM program selection evidence
- `dsp/gfm-v0.9-patch-voice-factory-evidence.md` - patch-to-GFM voice
  factory contract evidence
- `dsp/gfm-v1.0-engine-layer-evidence.md` - first offline-only engine voice
  graph GFM layer evidence
- `dsp/gfm-v1.1-layer-gate-evidence.md` - explicit engine-owned GFM layer gate
  evidence
- `dsp/gfm-v1.2-standalone-runtime-flag-evidence.md` - standalone launch-time
  GFM layer flag evidence
- `dsp/gfm-v1.3-gui-runtime-toggle-evidence.md` - windowed GUI GFM runtime
  toggle evidence without full audio runtime rebuild
- `dsp/gfm-v1.4-midi-performance-controls-evidence.md` - MIDI aftertouch and
  K8/CC3 GFM amount control evidence
- `dsp/gfm-v1.5-momentary-midi-gate-evidence.md` - armed GFM layer with
  aftertouch momentary gate evidence
- `dsp/gfm-v1.6-midi-auto-arm-evidence.md` - K8/CC3 GFM auto-arm evidence for
  momentary MIDI performance without GUI pre-enable
- `dsp/gfm-v1.7-smoothed-momentary-color-evidence.md` - smoothed GFM
  aftertouch/K8 color entry and release evidence
- `dsp/gfm-v1.8-swapped-k8-aftertouch-controls-evidence.md` - swapped K8 gate
  and aftertouch amount control evidence
- `dsp/bcs-v0.1-hopf-duffing-playground-evidence.md` - first offline
  Hopf/Duffing BCS playground render evidence
- `dsp/bcs-v1.0-engine-layer-smoke-evidence.md` - disabled-by-default BCS
  engine layer smoke evidence
- `dsp/bcs-v1.1-standalone-runtime-flag-evidence.md` - standalone dry-run/play
  BCS scenario flag and interactive command evidence
- `dsp/bcs-v1.2-playable-midi-layer-evidence.md` - playable BCS MIDI layer
  evidence for PC4 `S9` amount and `SW9` enable

Current source-doc posture:

- local `EPM1` implementation truth lives in this repo's code, runtime, patches,
  and top-level README
- local transport architecture work is now frozen at the current bridge state;
  future transport evolution is expected to move into the shared Linux audio
  platform documentation track
- the current shared software/source architecture docs that shaped `EPM1`
  still live in the sibling `mamut-sint-hw` docs track
- the current shared Linux audio platform direction now lives in
  the sibling `mamut-platform` docs track

Current `EPM1` source anchors:

- sibling `mamut-sint-hw`: `docs/software-clone-architecture.md`
- sibling `mamut-sint-hw`: `docs/rust-workspace-architecture.md`
- sibling `mamut-sint-hw`: `docs/dsp-subsystem-spec.md`
- `dsp/primitives-math.md`
- `dsp/render-path-math.md`
- `dsp/control-identity-math.md`
- sibling `mamut-sint-hw`: `docs/patch-schema-v1.md`
- `../mamut-platform/docs/LINUX_AUDIO_PLATFORM_THESIS.md`
- `../mamut-platform/docs/LINUX_AUDIO_STACK_DECISION.md`
- `../mamut-platform/docs/C_RUST_BOUNDARY.md`
- `../mamut-platform/docs/LINUX_AUDIO_OPEN_QUESTIONS.md`
- `../mamut-platform/docs/LINUX_AUDIO_TRANSPORT_ARCHITECTURE.md`
- `../mamut-platform/docs/EPM1_FREEZE_AND_RESUME.md`
