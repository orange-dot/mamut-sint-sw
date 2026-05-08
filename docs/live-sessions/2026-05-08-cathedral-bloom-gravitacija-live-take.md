# 2026-05-08 Cathedral Bloom Gravitacija Live Take

## Boundary

This note preserves the 60-second windowed/runtime live take captured from the
standalone direct output recorder.

- patch: `cathedral-bloom`
- patch name: `Cathedral Bloom`
- take tag: `gravitacija`
- recording mode: standalone direct f32 stereo WAV capture with MIDI sidecar
- sample rate: `96000 Hz`
- duration target: `60s`
- controller profile recorded by sidecar: `pc4-legacy`
- MIDI channel recorded by sidecar: `all`
- operator result: accepted as a good-sounding live take, then requested for
  full analysis and evidence capture

Artifacts:

```text
<lab-root>/audio-captures/cathedral-bloom-gravitacija-60s-20260508-210754.wav
<lab-root>/audio-captures/cathedral-bloom-gravitacija-60s-20260508-210754.midi.log
```

Hashes:

```text
d88e47bedb6446e731a8c1253486bb7fedf4aa370789c8f81c54ad866045ddca  cathedral-bloom-gravitacija-60s-20260508-210754.wav
abe7b044ce4fa3956efb63f0f55c91b847bd3449047e8b2fe735b83d9a8487d6  cathedral-bloom-gravitacija-60s-20260508-210754.midi.log
```

## Source Metadata

File inspection:

```text
cathedral-bloom-gravitacija-60s-20260508-210754.wav:
  RIFF little-endian WAVE, IEEE Float, stereo, 96000 Hz

cathedral-bloom-gravitacija-60s-20260508-210754.midi.log:
  ASCII text, 862 lines
```

MIDI sidecar header:

```text
patch_name: Cathedral Bloom
patch_path: patches/factory/cathedral-bloom.toml
sample_rate_hz: 96000
max_frames: 5760000
midi_channel: all
controller_profile: pc4-legacy
```

## Audio Analysis

The WAV is technically clean.

```text
sample_rate: 96000 Hz
channels: 2
frames: 5760000
duration: 60.000000 s
finite samples: 11520000 / 11520000
```

Level metrics:

```text
peak_L:        0.812965  -1.80 dBFS
peak_R:        0.830749  -1.61 dBFS
overall_peak:  0.830749  -1.61 dBFS
rms_L:         0.270640 -11.35 dBFS
rms_R:         0.194559 -14.22 dBFS
overall_rms:   0.235690 -12.55 dBFS
crest_factor: 10.94 dB
dc_L:         +0.00010999 -79.17 dBFS
dc_R:         +0.00013869 -77.16 dBFS
```

Safety/headroom:

```text
abs >= 0.999: 0 / 0 samples
abs >= 0.980: 0 / 0 samples
abs >  1.000: 0 / 0 samples
```

Stereo:

```text
stereo_correlation:     0.8489
L/R RMS balance:       +2.87 dB L over R
mid_rms:                0.223884
side_rms:               0.073661
side_vs_mid:           -9.66 dB
```

Interpretation: the take is mono-compatible and not excessively wide, but the
left channel is materially louder than the right channel. Keep this as a
follow-up check if the imbalance was not intentional for this patch/take.

## Spectrum

The take is dark, massive, and low/low-mid dominant.

```text
spectral_centroid: 356.1 Hz
rolloff_85:        527.3 Hz
rolloff_95:       1043.0 Hz

0-60 Hz:        23.537 %
60-250 Hz:      22.298 %
250-1000 Hz:    48.850 %
1000-4000 Hz:    4.884 %
4000-8000 Hz:    0.383 %
8000-16000 Hz:   0.046 %
16000-48000 Hz:  0.002 %
```

Strongest relative spectral peaks:

```text
58.6 Hz     0.0 dB
257.8 Hz   -1.7 dB
410.2 Hz   -3.4 dB
128.9 Hz   -6.0 dB
527.3 Hz   -7.8 dB
832.0 Hz   -8.1 dB
```

## Timeline

The MIDI sidecar `t=` clock is time from MIDI input open, not time from the WAV
start. Using the sidecar finish marker and the 60-second WAV duration, the WAV
start maps to approximately MIDI `t=242.012s`.

Mapped note timeline:

```text
03.488s  note_on   note=29  velocity=0.803
06.103s  note_on   note=30  velocity=0.591
12.281s  note_off  note=30
12.366s  note_off  note=29
23.592s  note_on   note=36  velocity=0.504
```

The final note `36` has no matching note-off before the recorder target elapsed.
Treat this as a performance tail/hold detail unless reproduced as a stuck-note
failure.

Five-second audio blocks:

```text
00-05s  rms=0.160669 -15.9 dBFS  peak=0.493903  centroid=103 Hz
05-10s  rms=0.259535 -11.7 dBFS  peak=0.607929  centroid=98 Hz
10-15s  rms=0.212023 -13.5 dBFS  peak=0.591770  centroid=106 Hz
15-20s  rms=0.018173 -34.8 dBFS  peak=0.108199  centroid=124 Hz
20-25s  rms=0.282376 -11.0 dBFS  peak=0.594630  centroid=154 Hz
25-30s  rms=0.275054 -11.2 dBFS  peak=0.557638  centroid=217 Hz
30-35s  rms=0.022771 -32.9 dBFS  peak=0.109336  centroid=108 Hz
35-40s  rms=0.140800 -17.0 dBFS  peak=0.470188  centroid=678 Hz
40-45s  rms=0.293003 -10.7 dBFS  peak=0.714079  centroid=528 Hz
45-50s  rms=0.257285 -11.8 dBFS  peak=0.605115  centroid=613 Hz
50-55s  rms=0.257008 -11.8 dBFS  peak=0.690848  centroid=521 Hz
55-60s  rms=0.263051 -11.6 dBFS  peak=0.660690  centroid=541 Hz
```

Highest 100 ms RMS windows clustered around `21.3-26.5s` at roughly
`-9.0 dBFS`. The absolute peak occurred in the right channel at `52.957625s`
with sample value `-0.830749`.

## MIDI Analysis

Event counts:

```text
total traced events: 847
note_on:              3
note_off:             2
mod_wheel:           83
pitch_bend:          89
macro:              315
ignored_cc:         355
```

Recognized control movement:

```text
mod_wheel:    83 events, 0.000-1.000, 08.284-09.173s
pitch_bend:   89 events, 0.000-2.000 st, 09.554-13.846s
macro_Bloom: 127 events, 0.000-1.000, 39.476-45.380s
macro_Heat:  188 events, 0.000-1.000, 41.098-57.249s
```

Ignored CC activity:

```text
cc3:   68 events, 0.000-1.000
cc9:   17 events, 0.000-0.535
cc24:  17 events, 0.543-0.929
cc25:  19 events, 0.299-0.693
cc26:  32 events, 0.000-0.504
cc27:  56 events, 0.150-1.000
cc28:  55 events, 0.543-0.992
cc71:  22 events, 0.236-1.000
cc72:  14 events, 0.575-1.000
cc73:  17 events, 0.000-0.417
cc79:  38 events, 0.000-0.732
```

Interpretation: the take name emphasizes `gravitacija`, but the sidecar shows
recognized macro performance mainly on `Bloom` and `Heat`. The dense ignored CC
traffic should be checked against the intended PC4 page/profile. If those were
intended sound controls, the mapping/focus layer did not claim them.

## Verdict

Accepted as a strong `Cathedral Bloom` live take and useful sound-character
reference.

Pass:

- valid 60-second f32 stereo WAV at `96000 Hz`
- all samples finite
- no clipping, no overs, and about `1.6 dB` peak headroom
- low DC offset after the output safety/DC-blocking work
- dense live controller movement captured in the sidecar
- tonal evidence matches the desired large/dark Cathedral Bloom character

Follow-ups:

- verify whether the `+2.87 dB` left-channel RMS dominance is patch character
  or a routing/output imbalance
- decide whether the final open note `36` is intentional performance sustain or
  a missed note-off case
- review the `355` ignored CC events against the current PC4 profile and Sound
  Lab focus rules
- if this take is used as public evidence, preserve the external audio artifact
  path or move/copy it into a stable artifact store; the WAV itself is not
  tracked in this repository
