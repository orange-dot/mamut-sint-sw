# GFM v2.3 INSPECT Field View Evidence

Date: 2026-07-07

Backlog: `EPM1_BACKLOG_SET1_GFM_PLAYABLE_FIELD.md`, item `SET1-3`.

The GFM innovation was invisible: `GfmDiagnostics` is scalar-only, and the
v1.9 field workbench was reverted from production. This slice promotes a
*bounded subset* of that machinery — a read-only terrain view on the
`INSPECT` screen — so an operator can watch the live field while the audible
gate stays closed. It adds no coupling-profile patch surface.

## What Landed

`mamut-field`:

- `GfmTerrainSnapshot<W, H>` (`GfmTerrainSnapshot16` for the production
  16×16) — a fixed-size, quantized, read-only snapshot: `energy`, `heat`,
  `fracture` as `[[u8; W]; H]` planes (energy/heat quantized across their
  cell ceilings, fracture across `0..1`), `health` as
  `[[GfmCellHealth; W]; H]`, a `recently_ruptured` boolean plane (cells whose
  rupture inhibition window is still open), the mono probe and the L/R stereo
  probe positions, and an 8-slot most-recent-first strike-marker ring
  (`GfmStrikeMarker { x, y, frame_index }`). Copying the snapshot allocates
  nothing.
- `GfmLattice::terrain_snapshot()` — one bounded read-only pass over the
  cells, no allocation, no RNG draw, no state change. Never called from the
  per-sample path. The lattice records a rolling ring of the last 8 strike
  positions (stamped with the frame index at injection) so the view can fade
  strike markers by age.

`mamut-engine`:

- `GfmFieldVoice::terrain_snapshot()` and
  `GfmLayerSnapshot::terrain: Option<GfmTerrainSnapshot16>`. The engine
  snapshot carries the terrain only while a GFM voice is armed; snapshot
  generation already runs off the audio fast path. `GfmCellHealth`,
  `GfmStrikeMarker`, `GfmTerrainSnapshot16`, and `GFM_TERRAIN_RECENT_STRIKES`
  are re-exported from `mamut-engine`.

`mamut-standalone`:

- An `INSPECT` screen (`crates/mamut-standalone/src/gui/inspect_methods.rs`)
  added to the tab strip alongside the legacy tabs, per the ADR 0003/0004
  four-screen IA cutover-in-place plan. When the GFM layer is disarmed it
  shows a one-line prompt to arm it; when armed it renders the 16×16 field
  with `egui::Painter`:
  - cell fill interpolates energy dark→cyan (`epm_panel_deep` → `epm_muted`
    → `epm_cyan`) with a warm shift by heat toward `epm_orange`
  - health borders: suspect `epm_orange_dim`, quarantined `epm_bad`,
    recovering `epm_ok`, healthy unbordered
  - recent-rupture cells carry a red centre dot
  - note-strike markers draw as orange rings that fade over ~1.6 s by frame
    age
  - the mono probe draws as a cyan cross; the L/R stereo probes draw as cyan
    rings labelled `L`/`R`
  - a header line shows program, strike count, max ruptures, and note-strike
    on/off; a legend explains the coding

No coupling-profile patch surface, no edit capability — read-only, exactly
the bounded subset the backlog scoped.

## Snapshot Path Evidence

- `mamut-field` `terrain_snapshot_is_read_only_and_matches_field_state` — a
  snapshotted lattice and a never-snapshotted twin, stepped over an identical
  excitation history, keep bit-identical mono output afterward and identical
  health histograms; the snapshot's frame index and probe positions match.
- `mamut-field` `terrain_snapshot_records_recent_strikes_most_recent_first` —
  after ten note strikes the ring holds the last eight in
  newest-first frame order, the newest marker is at the expected note-mapped
  cell, and that cell reads non-zero energy.
- `mamut-engine` `gfm_layer_snapshot_carries_terrain_only_while_armed` — the
  snapshot's `terrain` is `None` when disarmed, `Some` (full 256-cell health
  histogram, correct frame index, correct newest strike marker) after a
  note-on while armed, and `None` again after disarming.

## View Logic Evidence

`mamut-standalone` unit tests in `inspect_methods.rs`:

- `strike_marker_alpha_fades_monotonically_and_expires` — a fresh marker is
  fully opaque, a mid-age marker is dimmer but visible, an old marker is fully
  transparent; a zero sample-rate is handled without panic.
- `cell_fill_tracks_energy_and_heat` — fill brightness rises with energy and
  shifts warm with heat.
- `health_strokes_flag_only_unhealthy_cells` — only suspect/quarantined/
  recovering cells get a border; healthy cells do not.

## Windowed Capture (honest gap)

This measurement host is headless (no `DISPLAY`/`WAYLAND_DISPLAY`; see the
CIM measurement-host note). The `egui` window was **not** launched and no
screenshot was captured here. What is proven on this host is the full data
path (field → engine snapshot → GUI-visible `terrain`) plus the view's
pure render logic (color, fade, health coding) by unit test. The windowed
screenshot and the live "field evolves while the gate is closed" observation
belong to a rig session on a machine with a display; this doc should gain that
screenshot when the `INSPECT` screen is next run on the live rig. Until then,
treat the field-view *capability* as landed and unit-verified, and the
windowed visual as pending rig capture.

## Playbook Cross-Link

`EPM1_GFM_LIVE_BUG_PLAYBOOK.md` Pass 1 (Gate/Arm Silent Layer) now references
the `INSPECT` field view as a classification aid: a field that visibly evolves
while the sound stays dry distinguishes "armed but gated silent"
(`effective_amount = 0.0`) from a dead layer.

## Validation

Commands run (2026-07-07):

- `cargo fmt --all --check` — clean
- `cargo check --workspace --all-targets --locked` — clean
- `cargo test --locked -p mamut-field` (terrain subset green; full crate green
  in the workspace run)
- `cargo test --locked -p mamut-engine` (terrain subset green; full crate
  green in the workspace run)
- `cargo test --locked -p mamut-standalone` (inspect + tab tests green; full
  crate green in the workspace run)

## Boundary Compliance

- zero changes on the audio callback path — `terrain_snapshot()` is on-demand
  read-only work; strike-ring recording is the existing event-boundary strike
  path stamping a fixed-size ring (no per-sample cost)
- snapshot copy is fixed-size (16×16 `u8`/health planes plus an 8-slot marker
  ring); no per-frame heap growth in the engine snapshot path
- `mamut-field` remains external-dependency-free
- `SessionStateSource` read surface is unchanged in shape — the GUI reads the
  terrain off the already-published `EngineSnapshot`; no command surface, no
  new write path
- legacy tabs coexist with the new `INSPECT` screen until cutover per ADR 0004
