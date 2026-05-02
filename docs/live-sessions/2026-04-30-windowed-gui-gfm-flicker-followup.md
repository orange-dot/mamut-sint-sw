# 2026-04-30 Windowed GUI GFM Flicker Follow-Up

## Boundary

This note records a visual GUI issue seen after adding the windowed GFM live-take
controls.

- mode: release windowed GUI
- route: `PC4 -> mioXM DIN 1 -> Mamut EPM1 -> AG03`
- feature involved: GFM enable/seed panel
- user verdict: does not block playing
- artifact:
  `/home/dev/Videos/Screencasts/Screencast_20260430_171058.webm`

No runtime, DSP, patch, profile, or MIDI-map change is implied by this note.

## Video Evidence

The captured file is a short Spectacle WebM:

```text
path: /home/dev/Videos/Screencasts/Screencast_20260430_171058.webm
size: 122K
duration: 6.615s
video: VP9, 1919x1030, 60 fps
bitrate: about 150 kb/s
```

Visual inspection shows the Mamut window staying alive and structurally stable.
The visible problem is a small flicker/jitter around the GFM enable/status area
when GFM is switched on. The recording also contains the Spectacle overlay fade,
and the very low video bitrate can exaggerate shimmer on thin text and borders.

## Current Assessment

This is treated as GUI polish, not as an audio or synth-engine failure.

- PC4 playing remains usable.
- GFM enable is opt-in and still reaches the expected active program state.
- The issue does not explain stuck notes, DC offset, clipping, or MIDI mapping.
- It is safe to continue live playing and record this as a later GUI task.

Likely implementation causes to inspect later:

- the GFM seed/apply action currently rebuilds the runtime synchronously from
  the egui interaction path;
- dynamic status fields can change width while the top-level layout is being
  repainted;
- long transport counters and GFM status strings may cause small panel reflow.

## Deferred Fix Shape

For the later GUI polish pass:

- queue GFM seed enable/disable as a pending UI action instead of rebuilding
  directly from the render callback;
- show a stable `GFM RESTARTING` or `GFM APPLYING` state during the transition;
- give status tiles fixed dimensions;
- compact large counters, for example `5.8M` instead of long frame counts;
- split GFM status into fixed fields rather than one long changing line.

## Verdict

Logged as a non-blocking follow-up. It should be fixed before presenting the GUI
as polished, but it does not block live PC4-controlled playing.
