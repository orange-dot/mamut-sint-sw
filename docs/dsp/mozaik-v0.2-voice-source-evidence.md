# Mozaik v0.2 Voice-Source Integration Evidence

Date: 2026-07-12

Backlog: `EPM1_BACKLOG_SET5_ORBITA_MOZAIK_KOSAVA.md`, item `SET5-4`.
Builds on `mozaik-v0.1-quasicrystal-osc-evidence.md` (`SET5-3`), which owns
the primitive-level evidence (golden comb, detent collapse, phason flip law,
pitch-anchor table).

Mozaik joins the voice as a blendable oscillator source: per-voice
`QuasicrystalOsc` state, summed with the existing oscillator mix pre-filter,
so it is played like any Mamut sound — through velocity, envelopes, the
filter, and the existing safety chain. All controls are session-only; the
patch schema is untouched (`schema_version` stays 1).

## What Landed

`mamut-engine`:

- `MozaikMode { Disabled, Enabled { seed } }`, `Engine::set_mozaik_mode`,
  `Engine::set_mozaik_param(MozaikParam, f32)`, `MozaikSnapshot`, and
  `DEFAULT_MOZAIK_SEED = 0x4D6F_7A31` in `api.rs`, mirroring the
  `GfmLayerMode`/`BcsLayerMode` idiom; the mode logic lives in a new
  `engine/mozaik.rs` beside `layers.rs`/`lifecycle.rs`. `EngineSnapshot`
  gains a `mozaik` field.
- Per-voice `QuasicrystalOsc` plus its seeded phason base in `VoiceState` —
  always present (fixed-size state), gain-gated by the smoothed mix. On
  voice trigger (and when the mode is enabled over already-sounding voices)
  the oscillator is reseated to the layer seed folded with the voice slot
  (splitmix64-style, `helpers.rs::mozaik_voice_phason_q32`) plus the current
  phason offset, and takes the current slope/contrast targets.
- Insertion point: the per-voice sample is added to the source mix *after*
  the cross/ring source blend and *before* the pre-filter drive soft clip,
  so it rides the voice filter, both envelopes, and every downstream safety
  stage. It tracks the same `f0` expression as the spectral/additive
  sources (`note + bend + spread detune + pitch instability`) — no private
  tuning. While `Disabled` the render loop skips the source entirely (one
  branch per voice per frame), which is what makes the disabled render
  bit-identical rather than merely quiet.
- The five session controls (the cap), each `[0, 1]` with a `LinearSmoother`
  and hostile-input sanitization (non-finite → the control's default):
  `mix`; `slope` mapped to `sigma ∈ [0.45, 0.75]` with a ±0.004 detent snap
  *in the mapping* (nearest detent wins where windows overlap; golden beats
  `5/8` on exact ties by detent ordering); `contrast` mapped to
  `gamma ∈ [1.0, 2.2]`; `phason` (absolute offset, latched per tile by the
  primitive); `drift` mapped to an auto-phason rate `0.5 · drift²` cycles/s
  (`0` = frozen; f64 accumulator, wrapped, deterministic). When the smoothed
  slope lands exactly on a detent the engine pushes the exact Q32 detent
  constant, not an f32 rounding of it.
- On-enable defaults (also the `reset_controllers` restore point):
  `mix = 0.35`, `slope` at the golden detent, `contrast` at `gamma = tau`,
  `phason = 0`, `drift = 0`. Enable ramps the mix from silence (60 ms) —
  click-free; `reset_controllers` restores defaults with smoothing and
  clears the drift accumulator; `panic` keeps the mode and control targets
  (voices reconfigure on their next trigger).

`mamut-runtime` / `mamut-standalone` (the GFM layer route, exactly):

- `--mozaik [<seed>]` on `play` and `dry-run` (optional seed; a seed-shaped
  token — digits or `0x` hex — is consumed and must parse, anything else
  falls through to the patch positional and the default seed is used), wired
  through `build_audio_runtime` beside the GFM/BCS modes and reapplied on
  patch/audio switches from the session-tracked seed.
- Headless commands: `mozaik` (status line), `mozaik on [<seed>]`,
  `mozaik off`, `mozaik set <mix|slope|contrast|phason|drift> <0..1>`.
- `EngineCommand::SetMozaikMode` / `SetMozaikParam` ride the existing
  engine-command channel with reply channels, exactly like
  `SetGfmLayerMode`/`SetBcsLayerMode` — not the panic/reset priority path.
- `status` (and `dry-run`) print the `mozaik:` line from the snapshot;
  `help` documents the commands and the launch flag.

Tests: 9 engine tests (`crates/mamut-engine/src/tests/mozaik.rs`) covering
disabled bit-identity after an enable/disable round-trip, enabled-render
audibility and bit-determinism, hostile-value sanitization, detent-snap
rules including the golden/`5/8` overlap, `reset_controllers` restore,
drift accumulation, and per-slot phason-base distinctness; 4
standalone-side tests covering the CLI flag, the headless command parser,
the status line, and the engine-thread command round-trip.

## Pre-Slice Baseline (Rendered Before The Engine Changed)

Per the Execution Protocol, the baseline was rendered at the parent commit
`1ec7ddf` (the `SET5-3` landing), *before* any engine change, using this
example's baseline stage with the identical note script and render loop:

- Command: `cargo run --locked -p mamut-engine --example
  mozaik_engine_source_ab_render` (baseline stage, commit `1ec7ddf`)
- `base`: `c175bb8e0d9d7376` (rms `0.405365`, peak `0.723622`)
- `cutoff_low` (300 Hz pin): `31895f0ca1012765`
- `cutoff_high` (9 kHz pin): `b973f3e9f9a5ced2`

These three signatures are hardcoded in the final example, which hard-fails
if any Disabled render drifts from them — the bit-identity bar is
executable on every future run, not a one-time observation.

## Render Contract

- Command: `cargo run --release --locked -p mamut-engine --example
  mozaik_engine_source_ab_render`
- Source: `crates/mamut-engine/examples/mozaik_engine_source_ab_render.rs`
  (CI's release-smoke builds all engine examples)
- Patch `molten-horizon`, 48 kHz, block 256, 6 voices, 5 s script:
  NoteOn 48 @ 0.1 s, NoteOn 55 @ 1.0 s, NoteOff 48 @ 3.0 s,
  NoteOff 55 @ 4.0 s; stereo float WAVs to `target/mozaik-render/engine/`
- Environment: i7-4600U (4 threads, 8 GiB), Fedora kernel
  7.1.3-200.fc44.x86_64, rustc 1.96.0, release profile, run 2026-07-12
- All tables below from one release run of one binary. A debug-profile run
  of the same source printed identical signatures for every render
  (cross-profile determinism observation; its timing numbers are not
  comparable and are not recorded).

## A/B Renders

| Script | RMS | Peak | DC L / R | FNV-64 signature |
| --- | ---: | ---: | ---: | --- |
| `a_disabled` | `0.405365` | `0.723622` | `0.000576` / `0.000579` | `c175bb8e0d9d7376` |
| `b_golden` (defaults) | `0.382886` | `0.909744` | `0.000811` / `0.000778` | `c5d9263e8925b78a` |
| `b_half_detent` (σ = 1/2) | `0.407435` | `0.763865` | `0.000662` / `0.000653` | `e3db58ca48dd087b` |
| `b_drift` (drift 0.8) | `0.380667` | `0.927943` | `0.001073` / `0.001017` | `4d9c9ea0ce15d6af` |

- **Disabled bit-identity**: `a_disabled` and both cutoff-pinned Disabled
  renders match the pre-slice baseline signatures exactly
  (`matches_pre_slice=true` × 3). Also covered at unit level: an
  enable→disable round-trip renders bit-identically to a never-touched
  engine.
- **Audibility of bare `mozaik on`**: the Enabled-minus-Disabled difference
  RMS over the held window (2.0–3.0 s) is `0.333470` — not subtle. (The
  unit test enforces a floor of `0.005`; the measured value clears it by
  ~66×.) The enabled render trades a little RMS (`0.405 → 0.383`, pre-filter
  soft-clip interaction) for a higher peak (`0.724 → 0.910`) and a dense
  pulse-fabric texture inside the voice.
- **Slope changes the sound**: the `1/2`-detent render is a distinct
  signature and character (harmonic tile train) from the golden default.
- **DC is handled downstream, as stated in v0.1**: the primitive carries
  ~`0.22` DC at defaults, yet the engine output DC moves only
  `0.0006 → 0.0008` — the master `DcBlocker` absorbs the Mozaik DC as
  designed. No new DC handling was added or needed.

## Through The Filter And Envelope (Not A Lab Curiosity)

Cutoff pinned via `DirectParam FilterCutoffHz` on otherwise identical
renders; the Enabled-minus-Disabled difference signal isolates the Mozaik
contribution (plus its nonlinear interaction):

| Cutoff pin | Diff spectral centroid | Diff energy share > 2.5 kHz |
| --- | ---: | ---: |
| `300 Hz` | `265.2 Hz` | `0.003389` |
| `9000 Hz` | `534.9 Hz` | `0.018856` |

The voice filter audibly shapes the blend: opening the cutoff doubles the
difference-signal centroid and raises its high-band share 5.6×. The example
hard-fails if the low-cutoff centroid is not below the high-cutoff one.

Envelope gating: difference RMS `0.333470` during hold (2.0–3.0 s) falls to
`0.164264` in the final tail window (4.79–5.0 s, after both releases). The
tail is not silent — molten-horizon's release plus reverb legitimately ring
— but the decay direction is asserted in-example. Mozaik stops with the
voice; there is no free-running source.

## Drift On A Held Note

`drift = 0.8` (auto-phason ≈ 0.32 cycles/s): signature diverges from the
no-drift render with difference RMS `0.353003` — the fabric audibly
rearranges over the held notes. Pitch constancy under phason motion is a
primitive-level property measured in the v0.1 evidence (strongest peak
`272.06 → 272.06 Hz` over a full phason cycle); the engine adds no pitch
path on top of it, so the composition claim is: v0.1 measured
constant-pitch rearrangement, v0.2 shows the rearrangement arriving in the
full voice render. Unit level: the drift accumulator advances and wraps in
`[0, 1)`, and `reset_controllers` clears it.

## Determinism

Two fresh `b_golden` renders in one process: byte-identical float streams,
both `c5d9263e8925b78a` (the mandatory two-run FNV-64 equality). Unit
level: bit-determinism holds under mid-render control changes
(drift + contrast). Same seed + same event stream + same sample rate ⇒
bit-identical output; determinism across sample rates is not promised
(tile lengths are sample-rate-relative), same as v0.1.

## Per-Block Cost (Measured, Not Guessed)

Steady state, held six-voice chord, 400 warm-up blocks, 4000 timed blocks
of 256 frames at 48 kHz, release profile, this host:

| Mode | Mean per block | Per frame |
| --- | ---: | ---: |
| Disabled | `1166.52 µs` | `4556.7 ns` |
| Enabled (defaults) | `1201.71 µs` | `4694.2 ns` |

Delta: **`+35.19 µs` per block** ≈ `137 ns` per frame for six voices
(≈ `23 ns` per voice-frame), i.e. +3.0 % over the disabled render cost and
0.66 % of the 5.33 ms block budget at 48 kHz. The budget conversation
belongs to `SET5-7`/`SET2`; this slice only measures honestly. The RPi3B
target remains untested here.

## Headless Round-Trip

`cargo run --locked -p mamut-standalone -- dry-run molten-horizon --mozaik`
(run 2026-07-12, debug):

```text
mozaik: mode=enabled seed=0x4D6F7A31 mix=0.350 effective_mix=0.031 slope=0.560 sigma=0.618034 snapped=on contrast=0.515 gamma=1.618 phason=0.000 drift=0.000 drift_phason=0.000
```

`--mozaik 0xABC` reports `seed=0xABC`; without the flag the line is
`mozaik: mode=disabled`. (`effective_mix=0.031` is the 60 ms enable ramp
sampled after the dry-run's single 256-frame block — correct, not a bug.)
The interactive `mozaik` / `mozaik on|off` / `mozaik set` surface is parser-
and engine-thread-tested; a live `play` session exercises the same
`EngineCommand` path the tests drive.

## Tuned-In-Slice Ledger

- On-enable defaults (`mix 0.35`, slope golden, `gamma = tau`, phason 0,
  drift 0): **kept**. The binding constraint (bare `mozaik on` audible on a
  held dry-run note) is met with hold-window difference RMS `0.333`.
- Mix smoothing 60/90 ms attack/release, 30 ms control smoothing: **chosen
  in-slice** (mirrors the GFM/BCS smoothing magnitudes; no rendered
  evidence argued for other values).
- Drift rate mapping `0.5 · drift²` cycles/s: **chosen in-slice**; at
  `drift 0.8` the rearrangement is clearly audible (difference RMS `0.353`
  against no-drift) without reading as vibrato.
- Detent snap width ±0.004 σ: backlog value, **kept**; the overlap rule is
  unit-tested on both sides of the golden/`5/8` midpoint.

## Honest Caveats

- **Session-only state does not survive runtime rebuilds with its knob
  values**: a patch or audio-device switch rebuilds the engine and reapplies
  only the session's `--mozaik` seed with on-enable defaults (same posture
  as the GFM/BCS layer flags). Likewise `mozaik on` always restores the
  defaults; `mozaik set` before `mozaik on` is legal but overwritten by
  design (the backlog fixes the on-enable state).
- The `sound_lab` extension intent table (GFM/BCS enable hints in saved
  Sound Lab patches) does **not** carry Mozaik intent yet — left for the
  `SET5-7` docs-truth pass to decide; nothing in the schema or factory bank
  changed.
- The envelope-gating tail window still carries release/reverb energy
  (`0.164` vs `0.333` hold); the claim is "decays with the voice", verified
  directionally in-example, not "silent by 5 s".
- The enabled render's slight RMS drop is real: the blend passes through
  the pre-filter drive soft clip, so adding a source can compress the sum.
  Peak rises accordingly. This is the designed insertion, not a mixing
  accident.
- GUI exposure (including `INSPECT`) is untouched per the ADR 0004 boundary;
  the `EngineSnapshot.mozaik` field exists for a future read-only stretch.
- Cost numbers are this dev host, release profile, one configuration; they
  are inputs to the `SET5-7` cross-concept table, not a budget claim.

## Nearest Relatives

The novelty ledger lives in the v0.1 evidence (Bresenham cut-and-project
oscillator, slope morph, first-class phason). What v0.2 adds is
integration, and its relatives are internal: the engine-owned session-layer
idiom (`GfmLayerMode`, `BcsLayerMode`) and the pre-filter source insertion
that every existing Mamut source uses. Believed new here is only the
combination: a quasicrystal oscillator as a per-voice, envelope-and-filter-
ridden synth source with a live phason/drift axis — no claim beyond that.

## Validation

Commands run (2026-07-12):

- `cargo fmt --all --check` — clean
- `cargo test --locked` — full workspace green (9 new engine tests, 4 new
  standalone tests)
- `cargo run --locked -p mamut-engine --example mozaik_engine_source_ab_render`
  — debug run, all baseline checks and assertions green
- `cargo run --release --locked -p mamut-engine --example
  mozaik_engine_source_ab_render` — the recorded run; identical signatures
- `cargo run --locked -p mamut-standalone -- dry-run molten-horizon --mozaik`
  — status line above

WAV artifacts under `target/mozaik-render/engine/`: `a_disabled.wav`,
`b_golden.wav`, `b_half.wav`, `b_drift.wav`.

## Boundary Compliance

- No transport, MIDI-parsing, queue-payload, or callback change (Set 4
  territory untouched); the new commands ride the existing engine-command
  channel with the existing reply idiom, not the priority path
- No patch-schema growth; factory bank and live set untouched
- Allocation-free in steady state: fixed per-voice oscillator state, one
  enum branch per voice per frame when disabled, no allocation, logging,
  or blocking added to the render path; behavior is gain-gated (mix
  smoother), never allocation-gated
- Five session controls plus mode — at the cap, not beyond it
- Workspace lint posture untouched (`unsafe_code = "forbid"`, clippy
  panic/unwrap warnings clean)
