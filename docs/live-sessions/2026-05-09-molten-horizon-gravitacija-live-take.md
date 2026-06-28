# 2026-05-09 Molten Horizon Gravitacija Live Take

## Boundary

This note preserves the 60-second windowed/runtime live take captured from the
standalone direct output recorder.

- patch: `molten-horizon`
- patch name: `Molten Horizon`
- take tag: `gravitacija`
- recording mode: standalone direct f32 stereo WAV capture with MIDI sidecar
- sample rate: `96000 Hz`
- duration target: `60s`
- controller profile recorded by sidecar: `pc4-full`
- MIDI channel recorded by sidecar: `1`
- operator result: accepted as a new good-sounding live take, then requested for
  evidence capture and sonic analysis

Artifacts:

```text
<lab-root>/audio-captures/molten-horizon-gravitacija-60s-20260509-172357.wav
<lab-root>/audio-captures/molten-horizon-gravitacija-60s-20260509-172357.midi.log
```

Hashes:

```text
333cd2f1277415d0dc60287e7ba0d44cc8158d19085341cd5ba1e85a61f28936  molten-horizon-gravitacija-60s-20260509-172357.wav
109db42d53ef8b47623ddbfc00ecc6b1711b057cbb4e9fe851acde7489eab746  molten-horizon-gravitacija-60s-20260509-172357.midi.log
```

## Source Metadata

File inspection:

```text
molten-horizon-gravitacija-60s-20260509-172357.wav:
  RIFF little-endian WAVE, IEEE Float, stereo, 96000 Hz
  codec: pcm_f32le
  duration: 60.000000 s
  size: 46080044 bytes

molten-horizon-gravitacija-60s-20260509-172357.midi.log:
  ASCII text, 874 lines
```

MIDI sidecar header:

```text
patch_name: Molten Horizon
patch_path: patches/factory/molten-horizon.toml
sample_rate_hz: 96000
max_frames: 5760000
midi_channel: 1
controller_profile: pc4-full
finish: recording target duration elapsed
midi_lines: 859
```

## Audio Analysis

The WAV is technically clean and loud, with no clipping or over-range samples.

```text
sample_rate: 96000 Hz
channels: 2
frames: 5760000
duration: 60.000000 s
finite samples: 11520000 / 11520000
```

Level metrics:

```text
peak_L:        0.950175  -0.44 dBFS
peak_R:        0.715356  -2.91 dBFS
overall_peak:  0.950175  -0.44 dBFS
rms_L:         0.430950  -7.31 dBFS
rms_R:         0.283239 -10.96 dBFS
overall_rms:   0.364652  -8.76 dBFS
crest_factor:  8.32 dB
dc_L:         +0.00043202 -67.29 dBFS
dc_R:         +0.00010261 -79.78 dBFS
```

Safety/headroom:

```text
abs >= 0.999: 0 samples
abs >= 0.980: 0 samples
abs >  1.000: 0 samples
absolute_peak: 25.972792s, left channel, 0.950175
```

Stereo:

```text
stereo_correlation:     0.9897
L/R RMS balance:       +3.65 dB L over R
mid_rms:                0.356211
side_rms:               0.078003
side_vs_mid:          -13.19 dB
```

Interpretation: this take is highly mono-compatible and very strongly
correlated, but it is also materially left-heavy. Keep this as a follow-up
balance check if `Molten Horizon` is expected to present as centered direct
stereo.

## Spectrum

The take is a heavy low/low-mid drone centered around a low C performance
gesture, with controlled upper edge and little true high-frequency energy.

```text
spectral_centroid: 443.8 Hz
rolloff_85:        638.7 Hz
rolloff_95:       2185.5 Hz

0-60 Hz:        18.656 %
60-250 Hz:      57.517 %
250-1000 Hz:    10.188 %
1000-4000 Hz:   12.787 %
4000-8000 Hz:    0.736 %
8000-16000 Hz:   0.113 %
16000-48000 Hz:  0.003 %
```

Strongest relative spectral peaks:

```text
64.5 Hz     0.0 dB
128.9 Hz   -0.5 dB
117.2 Hz   -3.5 dB
29.3 Hz    -3.5 dB
193.4 Hz   -8.2 dB
99.6 Hz    -9.6 dB
263.7 Hz   -9.6 dB
164.1 Hz  -10.2 dB
228.5 Hz  -10.9 dB
175.8 Hz  -11.7 dB
```

Interpretation: the main body sits in the fundamental/second-harmonic region,
with enough `1-4 kHz` energy to read as pressure and edge rather than a pure
sub drone. The very small `8 kHz+` share confirms the character is dark and
massive, not fizzy.

## Timeline

The MIDI sidecar `t=` clock is time from MIDI input open, not time from the WAV
start. For this take the first real audio onset occurs at `7.864s`, matching
the first recorded note-on. The mapped timeline below uses that onset as the
audio alignment anchor.

Mapped note timeline:

```text
07.864s  note_on   note=36  velocity=0.858
14.525s  note_off  note=36
14.589s  note_on   note=36  velocity=0.874
16.676s  note_off  note=36
20.779s  note_on   note=36  velocity=0.819
25.087s  note_off  note=36
25.416s  note_on   note=36  velocity=0.898
29.667s  note_off  note=36
29.998s  note_on   note=36  velocity not released before recorder stop
```

The last `note=36` remains open through the end of the capture. In this take it
reads as an intentional held tail unless reproduced as a stuck-note failure.

Five-second audio blocks:

```text
00-05s  rms=-180.4 dBFS  peak=0.000000  centroid=n/a
05-10s  rms= -11.7 dBFS  peak=0.946180  centroid=377 Hz
10-15s  rms=  -8.1 dBFS  peak=0.949788  centroid=371 Hz
15-20s  rms= -12.9 dBFS  peak=0.947360  centroid=425 Hz
20-25s  rms=  -8.2 dBFS  peak=0.946524  centroid=430 Hz
25-30s  rms=  -8.6 dBFS  peak=0.950175  centroid=418 Hz
30-35s  rms=  -7.9 dBFS  peak=0.950050  centroid=424 Hz
35-40s  rms=  -7.2 dBFS  peak=0.860796  centroid=480 Hz
40-45s  rms=  -7.2 dBFS  peak=0.866348  centroid=460 Hz
45-50s  rms=  -8.0 dBFS  peak=0.943785  centroid=466 Hz
50-55s  rms=  -7.9 dBFS  peak=0.948268  centroid=486 Hz
55-60s  rms=  -7.8 dBFS  peak=0.941458  centroid=479 Hz
```

Highest 100 ms RMS windows:

```text
41.2-41.3s  -6.87 dBFS  peak=0.783990
22.3-22.4s  -6.89 dBFS  peak=0.854541
22.4-22.5s  -6.90 dBFS  peak=0.823623
38.5-38.6s  -6.91 dBFS  peak=0.719811
42.7-42.8s  -6.91 dBFS  peak=0.757384
```

## MIDI Analysis

Event counts:

```text
total traced events:      859
note_on:                    5
note_off:                   4
mod_wheel:                502
pitch_bend:               245
aftertouch:                 0
profile events:           103
direct_param events:       98
sound_lab_overlay events: 388
macro events:             108
```

Recognized control movement:

```text
mod_wheel:   502 events, 0.000-1.000, mapped 11.376-45.136s
pitch_bend:  245 events, -2.000-0.000 st, mapped 24.883-48.277s
K1:           21 events, mapped 53.050-54.231s
K2:           33 events, mapped 51.312-55.453s
K3:           49 events, mapped 56.214-57.916s
```

Sound Lab overlay movement:

```text
Osc 1 / MW / Osc1 Saw Bend:
  285 events, -1.000..1.000, last -1.000

Osc 1 / K2 / Osc1 Pulse Level:
  33 events, 0.220..0.449, last 0.449

Osc 1 / K1 / Osc1 Saw Level:
  21 events, 0.386..0.543, last 0.543

Osc 1 / K3 / Osc1 Triangle Level:
  49 events, 0.189..0.449, last 0.189
```

Macro movement:

```text
Bloom:       66 events, 0.220..0.449, last 0.449
Gravitacija: 42 events, 0.386..0.543, last 0.543
```

Interpretation: the performance is not a chordal or melodic take. It is a
single low-C pressure study: repeated attacks establish the low body, then the
long held note is shaped through mod wheel, downward pitch-bend, and late
Oscillator 1 source-level edits. `Gravitacija` is present, but the take's
dominant live gesture is the MW/pitch-bend mass movement.

## Verdict

Accepted as a strong `Molten Horizon` live take and useful evidence for the
current Mamut character target: dense, controlled, dark, and physically heavy
without clipping. The main technical follow-up is stereo balance: this capture
is clean, but strongly left-biased and much more mono-correlated than wide.
