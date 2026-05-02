# Live Session Evidence

These notes preserve real lab evidence from named hardware runs.

Exact device names, ALSA selectors, raw MIDI bytes, and transport counters are
kept because they explain what was observed during the session. Treat them as
evidence for those runs, not as portable defaults for every machine.

Portable launch instructions live in the top-level `README.md` and the focused
playbooks under `docs/`.

Recent sessions:

- `2026-04-30-windowed-gui-gfm-flicker-followup.md` - non-blocking visual
  flicker/jitter observed while enabling GFM from the windowed GUI, recorded as
  later GUI polish rather than an audio or MIDI blocker.
- `2026-04-30-gfm-cathedral-bloom-gravitacija-headless.md` - first short
  headless GFM runtime-flag take on Cathedral Bloom with Gravitacija movement,
  including the 30-second direct recorder artifact, hash, and final runtime
  status.
- `2026-04-30-direct-output-dc-blocker-validation.md` - follow-up direct
  output capture after the master DC blocker, validating that the large DC
  offset is gone and capturing the next headroom/denormal follow-ups.
- `2026-04-30-direct-output-capture-analysis.md` - first analysis of the new
  direct Mamut stereo WAV capture path, confirming valid f32 WAV output and
  capturing the large DC offset follow-up.
- `2026-04-30-pc4-ag03-external-camera-solo-synth.md` - release windowed GUI
  take recorded externally, launched on `Razor Thaw`, then used as a short
  factory-patch exploration with clean shutdown.
- `2026-04-29-pc4-ag03-windowed-gui-redesign-live-take.md` - EPM-style
  windowed GUI redesign take, including the first layout regression, the fixed
  top bar retest, and real PC4 note/controller evidence.
- `2026-04-29-pc4-ag03-windowed-live-jam-startup-guard.md` - windowed live jam
  after the MIDI startup guard, with patch switching and full controller
  evidence.
- `2026-04-29-pc4-ag03-full-controller-map.md` - one-way PC4 controller map
  validation for knobs, sliders, switches, wheels, sustain, and aftertouch.
