# 2026-04-30 GFM Cathedral Bloom Gravitacija Headless Take

## Boundary

This note locks the first short headless live take for the standalone GFM
runtime flag after Slice N.

- patch: `cathedral-bloom`
- performance focus: Cathedral Bloom with Gravitacija movement
- duration target: `30s`
- mode: headless standalone, direct internal recorder
- audio device: `hw:4,0` (`AG06/AG03, USB Audio`)
- MIDI device: `mioXM:mioXM DIN 1 28:0`
- MIDI channel: `1`
- controller profile: `profiles/pc4-full.toml`
- GFM seed: `0x6A464D40`
- selected GFM program: `HorizontPerformance`
- active GFM program: `HorizontPerformance`
- selection scores: `(0.8994,0.1359,0.0106)`

The artifact path is local lab evidence, not a portable default:

`target/gfm-live/cathedral-bloom-gravitacija-30s.wav`

## Commands Used

Launch:

```bash
cargo run --release -p mamut-standalone -- play --headless --audio-device hw:4,0 --midi-device 1 --midi-channel 1 --controller-profile profiles/pc4-full.toml --trace-midi --gfm-layer-seed 0x6A46_4D40 cathedral-bloom
```

Recorder command inside the headless prompt:

```text
record 30 target/gfm-live/cathedral-bloom-gravitacija-30s.wav
```

Inspection:

```bash
file target/gfm-live/cathedral-bloom-gravitacija-30s.wav
sha256sum target/gfm-live/cathedral-bloom-gravitacija-30s.wav
ls -lh target/gfm-live/cathedral-bloom-gravitacija-30s.wav
ffprobe -hide_banner -show_format -show_streams target/gfm-live/cathedral-bloom-gravitacija-30s.wav
ffmpeg -hide_banner -i target/gfm-live/cathedral-bloom-gravitacija-30s.wav -af astats=metadata=1:reset=0 -f null -
```

## Runtime Status

Mid-take status showed active held notes and no dropped recording frames:

```text
gfm: mode=enabled seed=0x6A464D40 selected=HorizontPerformance active=HorizontPerformance scores=(0.8994,0.1359,0.0106) ruptures=0
voices: active=6 sustain=false held=[69, 66, 62, 62, 67, 74] peak=0.668 clip=false
recording: recording path=target/gfm-live/cathedral-bloom-gravitacija-30s.wav written_frames=539392 dropped_frames=0 target_frames=1323000
macros: gravitacija=0.354 bloom=0.803 heat=0.220 ruin=0.160 swarm=0.580
identity: horizont_open=0.908 pec_mass=0.145 baklja_ready=0.051 grav_pull=0.252
```

Final status after the 30-second target completed:

```text
gfm: mode=enabled seed=0x6A464D40 selected=HorizontPerformance active=HorizontPerformance scores=(0.8994,0.1359,0.0106) ruptures=0
voices: active=0 sustain=false held=[] peak=0.001 clip=false
midi activity: messages=316
transport: queued=256 target=512 write_hint=256 underrun_batches=22740 underrun_frames=5821440 xrun_recoveries=3 overflow_batches=0 overflow_frames=0
recording: done path=target/gfm-live/cathedral-bloom-gravitacija-30s.wav written_frames=1323000 dropped_frames=0 target_frames=1323000
macros: gravitacija=0.134 bloom=0.803 heat=0.220 ruin=0.080 swarm=0.580
identity: horizont_open=0.957 pec_mass=0.102 baklja_ready=0.000 grav_pull=0.079
derived: mass=0.109 strain=0.050 headroom=0.942 threshold=0.807
```

The live trace included Gravitacija/K1 sweeps from about `0.465` to `1.000`,
down to about `0.134`, plus Bloom/K2 movement, note on/off events, pitch bend,
aftertouch, and direct PC4 controller traffic.

## Artifact Evidence

Format:

```text
target/gfm-live/cathedral-bloom-gravitacija-30s.wav: RIFF (little-endian) data, WAVE audio, IEEE Float, stereo 44100 Hz
codec: pcm_f32le
sample_fmt: flt
channels: 2
sample_rate: 44100
duration: 30.000000
duration_ts: 1323000
size: 10584044 bytes
bit_rate: 2822411
```

File size and hash:

```text
-rw-r--r--. 1 dev dev 11M Apr 30 16:28 target/gfm-live/cathedral-bloom-gravitacija-30s.wav
27c92b469efbd881c2337ec7cedb84566ac0752c3b03a58c018df0d7f9d8660f  target/gfm-live/cathedral-bloom-gravitacija-30s.wav
```

Audio integrity metrics from `ffmpeg astats`:

```text
overall peak: -2.899711 dB
overall RMS: -19.669862 dB
overall DC offset: -0.000080
min level: -0.539412
max level: 0.716167
samples: 1323000
NaN: 0
Inf: 0
denormals: 0
```

## Verdict

Evidence locked for this 30-second Cathedral Bloom / Gravitacija take:

- GFM runtime flag was active with the expected deterministic seed.
- Patch selection stayed on `HorizontPerformance`.
- Recorder produced the full target duration.
- Recording dropped frames were `0`.
- Artifact is finite, bounded, and below full-scale sample peak.

Residual live-host caveat:

- Final transport status recorded `xrun_recoveries=3` and high underrun
  counters. This does not invalidate the recorded artifact, but it remains
  relevant for later ALSA/headless stability work.

Operator direction after the take: evidence locked; proceed toward Slice P.
