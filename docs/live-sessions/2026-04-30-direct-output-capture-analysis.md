# 2026-04-30 Direct Output Capture Analysis

## Boundary

This note captures the first analysis pass for the new direct Mamut output
capture path. The goal was not to judge a musical take in detail, but to verify
that the runtime-created WAV is structurally valid and to identify obvious audio
quality issues for later work.

- artifact:
  `/home/dev/work-base-20260421/audio-captures/mamut-output-20260430-005500.wav`
- capture source: direct final stereo output from `mamut-standalone`
- expected route: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- capture format target: stereo 32-bit float WAV at 44.1 kHz
- duration target: `70s`
- operator note: the run was a direct Mamut capture, not AG03 loopback or
  Spectacle desktop recording

The concrete absolute artifact path is preserved as local lab evidence. It is
not a portable path.

## Commands Used

Format inspection:

```bash
ls -lh /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-005500.wav
file /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-005500.wav
ffprobe -hide_banner -show_format -show_streams /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-005500.wav
```

Audio metrics:

```bash
ffmpeg -hide_banner -i /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-005500.wav -af astats=metadata=1:reset=0 -f null -
ffmpeg -hide_banner -i /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-005500.wav -af ebur128=peak=true:framelog=quiet -f null -
ffmpeg -hide_banner -i /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-005500.wav -af silencedetect=n=-60dB:d=0.5 -f null -
ffmpeg -hide_banner -i /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-005500.wav -af silencedetect=n=-50dB:d=0.5 -f null -
```

A small stdlib Python scan was also used to verify per-sample min/max, RMS, DC
mean, clipping counts, 100 ms activity windows, and stereo correlation. `sox`,
`soxi`, and `numpy` were not available in the local environment.

## Format Evidence

The file is a valid direct-output WAV:

```text
RIFF WAVE audio, IEEE Float, stereo 44100 Hz
codec: pcm_f32le
sample_fmt: flt
channels: 2
sample_rate: 44100
duration: 70.000000
frames: 3087000
size: 24696044 bytes
bit_rate: about 2822 kb/s
```

This confirms that the direct recorder wrote the expected 70-second stereo
float capture and that the generated file is readable by standard tools.

## Level And Integrity Metrics

Summary from `ffmpeg astats`, `ebur128`, and the Python sample scan:

```text
peak:       L -3.00 dBFS, R -3.00 dBFS
true peak:  -2.6 dBFS
RMS:        L -6.78 dBFS, R -7.43 dBFS
overall RMS -7.09 dBFS
LUFS-I:     -5.0 LUFS
LRA:         1.5 LU
crest:      L 3.78 dB, R 4.43 dB
min:        L -0.669140, R -0.665044
max:        L +0.707946, R +0.707946
NaN:        0
Inf:        0
denormal:   0
>=0.999:    L 0, R 0
>1.0:       L 0, R 0
```

The capture does not clip and contains no invalid floating-point samples. It is
very loud for a raw synth capture, but still has roughly 2.6 dB true-peak
headroom.

## Activity And Timing

Windowed activity scan:

```text
active above -60 dBFS: 70.0s / 70.0s
active above -50 dBFS: 70.0s / 70.0s
active above -36 dBFS: 70.0s / 70.0s
loudest 100 ms window: 48.1s-48.2s at -4.46 dBFS
start 1s RMS: -25.99 dBFS
end 1s RMS:   -7.86 dBFS
quiet runs >= 0.5s below -60 dBFS: none
```

The first roughly three seconds are lower-level material, around `-23` to
`-29 dBFS` in 100 ms windows, then the take enters the full performance level.
The last second is still active, so the capture stopped while the synth was
still ringing or being played.

The generated filename uses UTC-style timestamping in the current implementation;
the local file modification time was about two hours later in the Belgrade
timezone.

## Stereo Evidence

Stereo relationship from the Python scan:

```text
uncentered correlation: 0.9497
centered correlation:   0.9406
AC mid RMS:             -7.98 dBFS
AC side RMS:            -22.86 dBFS
side vs mid:            -14.88 dB
```

The capture is stereo, but strongly centered. The side channel is present and
usable, just much quieter than the mid channel.

## Main Finding

The important technical finding is the large positive DC offset:

```text
DC mean L: +0.176555
DC mean R: +0.174700
```

That is too high for a final audio output. It does not clip this capture, but it
does consume headroom and can cause clicks or downstream export/playback issues.

Likely next investigations:

- determine whether the offset is introduced by a specific DSP stage, patch
  parameter combination, final mix stage, or capture path
- add a master-output DC blocker or very gentle high-pass filter if the offset
  is systemic
- decide whether capture should preserve raw engine output or capture the
  same DC-protected master sent to ALSA
- add a regression check that long direct captures keep DC mean near zero

## Verdict

Pass for recorder mechanics:

- direct Mamut stereo capture was produced
- duration and sample format match the requested `70s`, stereo, 44.1 kHz, f32
  WAV target
- no clipping, invalid floats, denormals, or file-structure failures were found

Follow-up required for audio quality:

- the master/capture path has a significant positive DC offset
- the take is very loud and low dynamic range for raw material (`-5.0 LUFS`,
  `1.5 LU` LRA)
- the recording ends while audio remains active, so later musical takes should
  include a stop/release tail if the artifact is meant for listening evidence
