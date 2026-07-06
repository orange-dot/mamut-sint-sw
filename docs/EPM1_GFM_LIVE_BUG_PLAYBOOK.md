# EPM1 GFM Live Bug Playbook

This playbook is the shortest real-rig path for separating three different
GFM failure classes that can sound similar during performance:

1. armed GFM but no audible layer because the live gate stays closed
2. controller mapping/focus loss, where the performer moves the intended GFM
   control but the runtime ignores or remaps it
3. host underrun/xrun under GFM-enabled live load

The point is not to "jam and guess". The point is to produce one short piece of
evidence for each candidate so we can classify the failure correctly.

## Rig

Use the locked Sprint 6 path unless the host requires a known equivalent:

- `PC4 -> mioXM DIN 1 -> EPM1 -> AG03/AG06`
- controller profile: `profiles/pc4-full.toml`
- MIDI channel: `2`
- use `--trace-midi` for every repro pass

Recommended patches:

- `cathedral-bloom` for the original Gravitacija/GFM line
- `molten-horizon` as a second heavy live patch

## Synthetic `mamut-seq` Rig (development-time)

Each pass below can be run either on the physical PC4 rig or on the synthetic
`mamut-seq` rig — the laptop MIDI sequencer (`crates/mamut-seq`) sending scripted
gestures over a virtual port. The scripted equivalents are:

- Pass 1 (Gate/Arm) — `scenarios/gfm-gate-arm.toml`
- Pass 2 (Mapping/Focus) — `scenarios/gfm-mapping-sweep.toml`
- Pass 3 (Host Load) — `scenarios/gfm-host-load.toml`

Run the two-process flow with `tools/run-seq-smoke.sh` (Mamut in one terminal,
`mamut-seq` in the other; see `docs/EPM1_BACKLOG_SET3_LAPTOP_MIDI_SEQUENCER.md`).

**Evidence class is not interchangeable.** A `mamut-seq` run is **synthetic
development evidence**: deterministic control streams, useful for development-time
classification and regression. It does **not** substitute for a real-rig run.
Final classification of a live-rig failure still requires the physical
`PC4 → mioXM → EPM1` path, and hardware evidence stays in `docs/live-sessions/`.
State which rig produced any evidence you save, and never file synthetic runs as
hardware evidence.

## Important Current Truth

Before running the playbook, hold these repo truths constant:

- `K8 / CC3` is still mapped as reserved no-op in the locked PC4 live profile.
  It is trace-visible, but not a claimed runtime control in the profile.
- GFM live audibility is gated by its control contract, not just by the
  enabled/disabled bit.
- High underrun counters are a host/runtime stability signal, not proof that
  the GFM DSP path is logically broken.

That means a performer can honestly experience "GFM is on, but nothing happens"
for more than one reason.

## Evidence To Save Every Time

For each pass, save:

- exact launch command
- patch name
- whether mode was `--headless` or `--windowed`
- one `status` capture before play
- one `status` capture during the target gesture
- one `status` capture after the pass
- raw MIDI trace excerpt showing the control movement
- audible verdict in one sentence
- whether counters grew: `underrun_*`, `xrun_recoveries`, `overflow_*`

If possible, also save:

- direct recorder WAV path
- sidecar MIDI log path

## Canonical Launch Commands

Headless:

```bash
tools/run-pc4-ag03.sh --trace-midi cathedral-bloom
```

Windowed:

```bash
tools/run-pc4-ag03.sh --windowed --trace-midi cathedral-bloom
```

Heavy host-load pass:

```bash
tools/run-pc4-ag03.sh --trace-midi gravity-wake \
  --alsa-period-frames 512 \
  --alsa-buffer-frames 2048 \
  --alsa-start-threshold-frames 2048
```

Inside the runtime, use:

```text
status
record 30 target/gfm-live/<tag>.wav
```

## Pass 1: Gate/Arm Silent Layer

Goal:

- prove whether the symptom is "GFM voice is armed but the audible gate stays
  closed"

Use:

- patch: `cathedral-bloom`
- mode: `--windowed` first, then repeat once in `--headless`

Procedure:

1. Launch with `--trace-midi`.
2. Capture initial `status`.
3. Put the runtime into whatever GFM-enabled state is currently intended for
   the session.
4. Hold one note or one simple chord.
5. Move only the control you believe should introduce GFM color.
6. Do one pass with no aftertouch.
7. Do one pass with clear aftertouch press and release.
8. Capture `status` after each phase.

Bug signal:

- runtime shows GFM as enabled/ready
- held notes are active
- performer moves the intended GFM control
- audible result stays identical or effectively dry
- trace shows no opening control for the audible gate, or snapshot/behavior
  implies `effective_amount` stayed at `0.0`

Expected healthy result:

- one phase stays dry when the gate is intentionally closed
- the aftertouch-open phase clearly blooms GFM into the sound
- releasing aftertouch returns to the base synth cleanly

Interpretation:

- if "GFM on but silent" happens only without aftertouch, this is likely not a
  DSP bug; it is the current gate contract
- if aftertouch is present but the layer still never opens, that is a real bug

## Pass 2: Mapping/Focus Loss

Goal:

- prove whether the intended GFM live control is being ignored, remapped, or
  claimed by another focus layer

Use:

- patch: `cathedral-bloom`
- mode: `--windowed`

Procedure:

1. Launch with `--trace-midi`.
2. Capture initial `status`.
3. Hold a stable note or chord.
4. Move one control at a time:
   - the control the operator believes is GFM amount
   - aftertouch
   - `K1`
   - `K2`
   - `mod wheel`
5. After each control movement, wait long enough to hear whether the sound
   changed and to read the trace.
6. Capture final `status`.

Bug signal:

- the trace shows raw CC traffic for the intended GFM control
- but the runtime logs it as ignored, reserved, or routes it to an unrelated
  macro/direct parameter
- audible behavior follows a different control than the performer expects

Expected healthy result:

- the trace and the sound agree about which control changes GFM-related color
- there is no dense unexplained `ignored cc` burst for the intended live control

Interpretation:

- if the operator uses `K8/CC3` expecting GFM amount, the current locked PC4
  profile itself is a likely root cause because `CC3` is still reserved no-op
- if another page/focus layer produces the expected sound movement while the
  intended page does not, this is a mapping/focus bug candidate

## Pass 3: GFM Host Underrun/Xrun

Goal:

- prove whether enabling and using GFM materially worsens transport stability

Use:

- patch: `gravity-wake` or `furnace-choir`
- mode: `--headless` first, `--windowed` second

Procedure:

1. Launch with `--trace-midi`.
2. Capture baseline `status`.
3. Play 60 to 120 seconds of ordinary live use:
   - repeated chords
   - sustain
   - aftertouch
   - the intended GFM color gesture
   - mod wheel
   - one patch change if safe
4. Capture mid-pass `status`.
5. Continue until either audible glitching appears or the pass completes.
6. Capture final `status`.
7. Repeat the same gesture set with GFM intentionally not introduced into the
   sound, if possible.

Bug signal:

- `underrun_batches`, `underrun_frames`, or `xrun_recoveries` keep climbing
  during ordinary GFM live use
- audible clicks or dropouts correlate with the counter growth
- the same host is materially cleaner when the GFM layer is not engaged

Expected healthy result:

- startup may show a small non-growing count
- ordinary play does not produce sustained counter growth
- no obvious audible glitching under the same gesture density

Interpretation:

- if counters grow only when the GFM color is actively exercised, this is a
  performance/stability issue
- if counters grow even with no GFM activity, the root cause is broader host
  transport instability

## Minimal Result Matrix

Fill this after the three passes:

| Pass | GFM enabled | Audible GFM? | Trace/control looked right? | Underruns grew? | Primary classification |
| --- | --- | --- | --- | --- | --- |
| Gate/Arm | yes/no | yes/no | yes/no | yes/no | gate contract bug or expected contract |
| Mapping/Focus | yes/no | yes/no | yes/no | yes/no | mapping/focus bug or operator mismatch |
| Host Underrun | yes/no | yes/no | yes/no | yes/no | performance issue or broader transport issue |

## Decision Rule

Classify the session like this:

- `enabled + silent + no opening aftertouch/gate evidence`:
  gate/arm contract issue
- `enabled + silent + intended CC appears ignored or rerouted`:
  mapping/focus bug
- `enabled + audible + counters climb + clicks/glitches`:
  host performance issue
- `enabled + offline evidence healthy + live only fails`:
  do not blame the core GFM DSP path first

## Good Follow-Up Artifacts

If a pass reproduces something important, lock a live-session note under:

- `docs/live-sessions/<date>-gfm-gate-repro.md`
- `docs/live-sessions/<date>-gfm-mapping-repro.md`
- `docs/live-sessions/<date>-gfm-host-underrun-repro.md`

Each note should include:

- boundary
- command
- relevant trace excerpt
- transport counters
- observed audio behavior
- verdict
