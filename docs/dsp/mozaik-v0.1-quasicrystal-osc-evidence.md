# Mozaik v0.1 Quasicrystal Oscillator Evidence

Date: 2026-07-12

Backlog: `EPM1_BACKLOG_SET5_ORBITA_MOZAIK_KOSAVA.md`, item `SET5-3`.

This is the first slice of the Mozaik concept: an oscillator whose waveform is
aperiodic yet perfectly ordered (cut-and-project / Fibonacci-word structure),
with a modulation axis — the phason — that periodic oscillators cannot have.
The slice is `mamut-dsp`-only: a new leaf module plus offline render evidence.
No engine, runtime, or patch-schema change of any kind; the engine
voice-source integration is `SET5-4`.

## What Landed

`crates/mamut-dsp/src/quasicrystal.rs`, exported via `lib.rs`:

- `QuasicrystalWord` — the Bresenham word core. State is one Q32 accumulator
  (`frac`) advanced per tile by `slope_q32`; the carry of
  `frac.overflowing_add(slope_q32)` is the tile kind (carry ⇒ `L`, no carry ⇒
  `S`). This computes `⌊(n+1)σ + φ⌋ − ⌊nσ + φ⌋` exactly in integers: zero
  floating-point drift, no tables, no allocation, no RNG. The phason `φ` is
  the accumulator's seat; shifting it is `frac.wrapping_add(Δφ)`.
- `QuasicrystalOsc` — the word core driving a train of full-tile Hann pulses,
  polarity `+` for `L` and `−` for `S`. Tile duration is
  `M · d_kind / d̄` with `d_S = 1`, `d_L = γ` (contrast),
  `d̄ = (1−σ) + σγ`, and `M = sample_rate / (2·f0)`, floored at
  `MOZAIK_MIN_TILE_SAMPLES = 4`. Hann endpoints have zero value and zero
  slope, so tile joins are C¹ by construction. Tile kind and length are
  computed only at tile boundaries; the fractional boundary remainder carries
  into the next tile so the mean tile rate stays exact.
- Phason changes (`set_phason`, `set_phason_q32`, `shift_phason_q32`) latch
  at tile boundaries via a pending target, so phason motion never splits a
  tile and is click-free by construction. `reset_to_phason_q32` is the
  voice-trigger reseat for the engine slice.
- Clamps: `σ ∈ [0.45, 0.75]` (Q32 `0x7333_3333..=0xC000_0000`),
  `γ ∈ [1.0, 2.2]` (default `τ`), `f0 ∈ [20, 8000]` Hz, `gain ∈ [0, 1]`.
  Non-finite control inputs fall back to the parameter default; non-finite
  `f0`/`sample_rate` fall back to `220 Hz` / `1 Hz`.
- Q32 slope detents as `const`s: `1/2` (`0x8000_0000`), `3/5` (`0x9999_999A`),
  `1/τ` (`0x9E37_79B9`), `5/8` (`0xA000_0000`), `2/3` (`0xAAAA_AAAB`). The
  rational detents are 2⁻³² approximations, so their words are
  astronomically-long-period approximations of periodic words; musically they
  are periodic. Stated here once, per the backlog, and not re-litigated.

Unit tests (`crates/mamut-dsp/src/tests.rs`):

- `quasicrystal_word_matches_floor_reference_over_1e5_tiles` — carry word
  equals the direct `⌊(n+1)σ+φ⌋ − ⌊nσ+φ⌋` u64 reference over 10⁵ tiles for
  all five detents × three phasons.
- `quasicrystal_long_tile_density_matches_slope_over_1e6_tiles` — L density
  equals `σ` within 10⁻³ over 10⁶ tiles for all detents.
- `quasicrystal_phason_latch_never_splits_a_tile` — two renders whose phason
  request lands at different samples inside the same tile are bit-identical
  everywhere (both latch at the same boundary), diverge from the unchanged
  render only after that boundary, and match it exactly through the end of
  the request tile.
- `quasicrystal_osc_clamps_hostile_parameters` — NaN/±inf/out-of-range on
  every setter and on `f0`/`sample_rate`; output stays finite and inside
  `[−1, 1]`.
- `quasicrystal_two_identical_runs_are_bit_identical`,
  `quasicrystal_reset_reseats_word_and_tile_state`,
  `quasicrystal_detent_half_word_alternates`,
  `quasicrystal_min_tile_floor_holds_at_high_f0`,
  `quasicrystal_word_slope_clamps_to_bounds`.

## Render Contract

- Command: `cargo run --locked -p mamut-dsp --example mozaik_v0_1_render`
- Source: `crates/mamut-dsp/examples/mozaik_v0_1_render.rs` (first
  `mamut-dsp` example; the float-WAV writer, FNV-64 PCM signature, and all
  spectral analysis are local to the example — an in-example radix-2 FFT,
  no external tools)
- Sample rate: `48_000 Hz`; mono IEEE-float WAV to `target/mozaik-render/`
- Base pitch: `f0 = 220 Hz` for all scenarios; defaults `σ = 1/τ`, `γ = τ`,
  `gain = 1`, phason `0` unless the scenario says otherwise
- Environment: i7-4600U (4 threads, 8 GiB), Fedora kernel
  7.1.3-200.fc44.x86_64, rustc 1.96.0, debug profile, run 2026-07-12
- All tables below transcribed from one run of one binary

## Scenario Metrics

| Scenario | Frames | RMS | Peak | FNV-64 signature |
| --- | ---: | ---: | ---: | --- |
| `golden` | `192000` | `0.612327` | `1.000000` | `e77442aec0c6693b` |
| `detent_walk` | `360000` | `0.612358` | `1.000000` | `b29eae9f73bd5ec3` |
| `slope_morph` | `288000` | `0.612345` | `1.000000` | `74ba4ce86b6200af` |
| `phason_drift` | `288000` | `0.612344` | `1.000000` | `a7dcadcc187ecfa6` |
| `contrast` | `360000` | `0.612390` | `1.000000` | `f9c7790ffb23bcce` |
| `determinism` | `192000` × 2 | — | — | `e77442aec0c6693b` (both runs) |

Every render is hard-checked in-example: any non-finite or out-of-unit
sample, any morph/drift step above the click guard, any pitch shift under
phason drift, or any determinism divergence fails the run.

### `golden` — stable discrete non-harmonic peaks

`σ = 1/τ`: DC mean `0.223249`, strongest peak `271.95 Hz`
(`1.2361 · f0`), top-8 peaks (dB re strongest):

```text
272.0:+0.0 168.1:-3.8 103.9:-9.4 207.8:-13.3 336.1:-13.3 711.9:-13.4 232.2:-13.9 64.2:-17.7
```

These are golden-comb lines: with tile rate `2·f0 = 440 Hz`, the peaks sit at
`440·σ = 271.9` (the L rate), `440·(1−σ) = 168.1` (the S rate),
`440·(2σ−1) = 103.9`, and further `ℤ + ℤσ` combinations — discrete, stable,
and not a harmonic series on any fundamental. Autocorrelation confirms order
without exact periodicity: best normalized autocorrelation `0.9860` at lag
`9709` (a Fibonacci near-period, ≈202 ms), against exactly `1.0000` for the
rational detents below. Quasiperiodicity means almost-periods approach 1
without reaching it; the detents' `1.0000` is the contrast that carries the
claim.

### `detent_walk` — harmonic collapse at rationals

Five steps of 1.5 s each, per-step FFT over the settled middle:

| Step | σ | Strongest peak | peak/f0 | autocorr | floor dB | Reading |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `1/2` | `0.500000` | `220.01 Hz` | `1.0000` | `1.0000` | `-189.5` | harmonic series on `f0` (220, 660, 880, 440, …) |
| `3/5` | `0.600000` | `264.01 Hz` | `1.2000` | `1.0000` | `-182.5` | harmonic series on `2·f0/5 = 88 Hz` |
| `golden` | `0.618034` | `271.94 Hz` | `1.2361` | `0.9923` | `-147.6` | golden comb (matches `golden` scenario) |
| `5/8` | `0.625000` | `275.00 Hz` | `1.2500` | `1.0000` | `-176.3` | harmonic series on `2·f0/8 = 55 Hz` |
| `2/3` | `0.666667` | `293.33 Hz` | `1.3333` | `1.0000` | `-184.4` | harmonic series on `2·f0/3 = 146.67 Hz` |

The collapse rule the walk demonstrates: at `σ = p/q` the word is periodic
with period `q` tiles spanning exactly `q·M` samples, so the spectrum is a
harmonic series on `2·f0/q`; between detents (golden) the comb is
non-harmonic and the autocorrelation peak drops below 1.

### `slope_morph` — continuous, click-free crystal↔quasicrystal morph

`σ` swept `0.45 → 0.75` over 6 s. Max sample-to-sample step `0.042131`
against `0.039787` for static golden and `0.042134` for static `σ = 0.75`:
the morph never steps outside what the static tile trains themselves do
(guard threshold `0.2`; a click would approach the pulse height). The morph
is audible as a continuous re-tiling, with the harmonic detents passing by.

### `phason_drift` — constant pitch, audible rearrangement

`σ = 1/τ` fixed, phason ramped `0 → 1` over 6 s (latched per tile).
Strongest peak first half `272.06 Hz`, second half `272.06 Hz` — constant
pitch to FFT resolution. Max step `0.039788` — identical to the static
render; the latch never clicks. Word-level flip fraction between phason `0`
and phason `Δ` over 20 000 tiles:

```text
Δ=0.01 → 0.0200   Δ=0.05 → 0.1000   Δ=0.10 → 0.2000   Δ=0.25 → 0.4999   Δ=0.50 → 0.7640
```

The measured flip fraction is `2Δ` for small `Δ` (each phason crossing swaps
an `LS`↔`SL` pair), saturating toward `2σ(1−σ)+…` at large shifts — the
rearrangement is real, graded, and pitch-neutral. This is the axis that makes
Mozaik Mozaik.

### `contrast` — timbre axis with measured DC

`γ` stepped `1.0 → 2.2` at golden slope:

| γ | DC mean | floor dB | RMS | Strongest peak |
| ---: | ---: | ---: | ---: | ---: |
| `1.000` | `0.118421` | `-148.2` | `0.612226` | `168.07 Hz` |
| `1.300` | `0.178358` | `-149.4` | `0.612349` | `271.95 Hz` |
| `1.618` | `0.224303` | `-148.0` | `0.612492` | `271.94 Hz` |
| `1.900` | `0.255328` | `-146.8` | `0.612594` | `271.94 Hz` |
| `2.200` | `0.280181` | `-144.7` | `0.612370` | `271.95 Hz` |

DC grows with `γ` exactly as the L/S asymmetry predicts (mean of a Hann tile
is half its height; DC `= 0.5·(σγ − (1−σ))/d̄`, e.g. `0.1184` at `γ = 1`).
At `γ = 1` the L and S tiles have equal length and the strongest line moves
to the S-rate comb line (`168 Hz`); from `γ ≈ 1.3` up the L-rate line
dominates. The spectral floor stays below `-144 dB` across the range.

### `determinism`

Two fresh `golden` renders: byte-identical, both signatures
`e77442aec0c6693b`. (Also covered at unit level, including under slope sweeps
and phason drift.)

## Pitch-Anchor Honesty Table

Golden defaults, 2 s per row, 48 kHz. Mozaik does not promise that perceived
pitch equals `f0`; it promises the relation is stable and stated. The
relation at golden defaults: **strongest peak = `σ·2·f0 = 2·f0/τ ≈
1.2361·f0`** (the L-tile rate line).

| f0 (Hz) | Strongest peak (Hz) | peak/f0 | Tile rate (Hz) | Nominal (2·f0) | Rate bias |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `55` | `67.96` | `1.2356` | `110.5` | `110` | `1.0045` |
| `110` | `135.95` | `1.2359` | `220.5` | `220` | `1.0023` |
| `220` | `271.95` | `1.2361` | `440.5` | `440` | `1.0011` |
| `440` | `543.86` | `1.2361` | `880.5` | `880` | `1.0006` |
| `880` | `1087.75` | `1.2361` | `1760.5` | `1760` | `1.0003` |
| `1760` | `2175.49` | `1.2361` | `3520.5` | `3520` | `1.0001` |
| `3520` | `4350.96` | `1.2361` | `7040.5` | `7040` | `1.0001` |
| `8000` | `4583.60` | `0.5729` | `12000.0` | `16000` | `0.7500` |

The anchor ratio is flat at `1.2361` from 55 Hz through 3520 Hz (the ≤0.5 %
rate bias at low `f0` is the one extra tile the 2 s window counts). At
`f0 = 8000` Hz and 48 kHz both tile kinds compute below the 4-sample floor,
so every tile is exactly 4 samples: the tile rate is `12 kHz` against the
nominal `16 kHz` (bias `0.75`, the recorded number the backlog demanded) and
the strongest peak folds to `4583.6 Hz ≈ 0.573·f0`. The top octave degrades
gracefully and predictably; nothing is hidden by the floor.

## Tuned-In-Slice Ledger

- `M = sample_rate / (2·f0)` (mean tile rate `2·f0`): **kept**. The measured
  anchor relation (`1.2361·f0`, stable across five octaves) is usable as a
  stated calibration; listening did not argue for moving `M` in v0.1. If the
  engine slice wants the strongest line *at* `f0`, the honest lever is a
  `σ`-dependent `M` recalibration there, recorded against this table.
- `MOZAIK_MIN_TILE_SAMPLES = 4`: **kept**; the measured `0.75` rate bias at
  `f0 = 8000`/48 kHz is the accepted cost, confined to the top octave.
- Analysis constants local to the example (FFT size 65 536, floor band
  18–24 kHz, click guard `0.2`): evidence-harness choices, not module
  parameters.

## Honest Caveats

- **DC is real and by design**: the `+L/−S` polarity convention with `γ > 1`
  leaves `0.12–0.28` DC at full gain across the contrast range (table above).
  The primitive does not hide it; the engine slice (`SET5-4`) blends Mozaik
  pre-filter into the voice path, where the existing per-voice/master
  `DcBlocker` chain is the stated answer. The evidence for that lands with
  `SET5-4`, not here.
- **Peak hits `1.0` exactly** at `gain = 1` (Hann apex). The primitive is
  full-scale; mixing headroom is the integrator's job (`mozaik_mix` in
  `SET5-4`).
- **`floor_db` bounds alias + leakage together**: it is the median FFT
  magnitude in the 18–24 kHz band relative to the strongest peak, from the
  Hann-windowed in-example FFT. It does not separate aliasing from window
  leakage; at `-144` to `-190 dB` the combined floor is far below audibility,
  which is the claim that matters. No polyBLEP needed — C¹ Hann joins do the
  work.
- **The golden autocorrelation reaches `0.986–0.992`** at Fibonacci
  near-period lags. That is what quasiperiodicity means (almost-periods
  arbitrarily close to 1); the discriminating evidence is the rational
  detents pinning `1.0000` while golden stays measurably below.
- **Determinism across sample rates is not promised.** Tile lengths are
  sample-rate-relative. Same settings + same call sequence + same sample rate
  ⇒ bit-identical output; that is the claim, verbatim.
- The FNV-64 signatures are over the 16-bit PCM projection of the float
  stream (house pattern), not the float bytes; the determinism scenario also
  compares the float stream bit-for-bit.

## Nearest Relatives

Established prior art, separated from what is believed new here (the
`masnoca.md` discipline):

- **Exists elsewhere**: golden-ratio FM and additive experiments; Fibonacci
  and Sturmian words as a mature mathematical object (cut-and-project
  sequences, three-distance theorem); low-discrepancy sequences in dithering;
  aperiodic tilings as visual art; Bresenham accumulators everywhere in
  graphics and DDS synthesis; quasicrystal diffraction spectra in condensed
  matter (the physical inspiration for the comb structure).
- **Believed new here**: a Bresenham cut-and-project *audio oscillator* in
  which the integer carry word is the waveform's structure; `σ` as a
  continuous crystal↔quasicrystal morph with rational detents as playable
  harmonic collapses; and the phason as a first-class, constant-pitch,
  click-free-by-construction modulation axis latched at tile boundaries. We
  found no instrument oscillator shipping this combination; the claim is
  "believed new", not "proven unprecedented".

## Validation

Commands run (2026-07-12):

- `cargo fmt --all --check` — clean
- `cargo test --locked -p mamut-dsp` — 57 tests green (9 new)
- `cargo test --locked` — full workspace green
- `cargo run --locked -p mamut-dsp --example mozaik_v0_1_render` — all
  in-example assertions green; every table above from that single run

WAV artifacts under `target/mozaik-render/`: `golden.wav`,
`detent_walk.wav`, `slope_morph.wav`, `phason_drift.wav`, `contrast.wav`.

## Boundary Compliance

- `mamut-dsp` stays zero-dependency; the module is pure safe Rust, no RNG,
  no I/O, no time source, no allocation anywhere in the oscillator (fixed
  scalar state; `Option<u32>` pending phason is fixed-size)
- no engine, runtime, transport, queue, or callback change; no patch-schema
  growth; factory bank and live set untouched
- workspace lint posture untouched (`unsafe_code = "forbid"`, clippy
  panic/unwrap warnings clean)
- Set 4 territory untouched (no MIDI, no transport, no queues)
