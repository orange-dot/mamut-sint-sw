# 2026-04-30 Direct Output DC Blocker Validation

## Boundary

This note validates the first real direct-output capture after the master DC
blocker was added to the `EPM1` engine output path.

- artifact:
  `/home/dev/work-base-20260421/audio-captures/mamut-output-20260430-125653.wav`
- capture source: direct final stereo output from `mamut-standalone`
- expected route: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- capture format target: stereo 32-bit float WAV at 44.1 kHz
- duration target: `70s`
- prior baseline:
  `audio-captures/mamut-output-20260430-005500.wav`
- local binary evidence:
  - debug `mamut-standalone`: `2026-04-30 14:53`
  - release `mamut-standalone`: `2026-04-30 14:54`
  - capture artifact: `2026-04-30 14:58`

The absolute paths and local times are preserved as lab evidence. They are not
portable defaults.

## Commands Used

Format inspection:

```bash
ls -lh /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-125653.wav
file /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-125653.wav
ffprobe -hide_banner -show_format -show_streams /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-125653.wav
```

Audio metrics:

```bash
ffmpeg -hide_banner -i /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-125653.wav -af astats=metadata=1:reset=0 -f null -
ffmpeg -hide_banner -i /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-125653.wav -af ebur128=peak=true:framelog=quiet -f null -
ffmpeg -hide_banner -i /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-125653.wav -af silencedetect=n=-60dB:d=0.5 -f null -
ffmpeg -hide_banner -i /home/dev/work-base-20260421/audio-captures/mamut-output-20260430-125653.wav -af silencedetect=n=-50dB:d=0.5 -f null -
```

A small Python RIFF scan was used because the standard library `wave` module did
not accept IEEE-float WAV format tag `3`.

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

This confirms that the direct recorder still writes the intended 70-second
stereo float artifact after the master-output DSP change.

## DC Offset Result

The DC blocker fixed the original high-offset failure.

Prior baseline:

```text
2026-04-30 00:55 capture
DC mean L: +0.176555
DC mean R: +0.174700
```

Current capture:

```text
full-file DC mean L: -0.000105889
full-file DC mean R: -0.000089248
after first 2s L:    -0.000013757
after first 2s R:    -0.000004951
```

The full-file mean is still slightly affected by startup/silence behavior, but
the active post-start material is effectively centered. This validates the
master DC blocker as part of the real direct capture flow, not only in unit
tests or synthetic probes.

## Level And Integrity Metrics

Summary from `ffmpeg astats`, `ebur128`, and the Python sample scan:

```text
peak:       L 0.00 dBFS, R 0.00 dBFS
true peak:  +0.3 dBFS
RMS:        L -6.22 dBFS, R -7.53 dBFS
LUFS-I:     -4.0 LUFS
LRA:         2.3 LU
min:        L -0.752257, R -0.633967
max:        L +1.000000, R +1.000000
NaN:        0
Inf:        0
sample >= 1.0: L 1061, R 46
sample <= -1.0: L 0, R 0
near clip >= 0.999: L 1082, R 46
```

The DC failure is gone, but this capture is hotter than the previous direct
capture. The positive sample peak reaches the f32 output clamp and the true peak
exceeds full scale. The next audio-quality item is therefore master headroom,
patch output trim, or a final limiter policy.

## Activity And Stereo Evidence

Silence detection found expected short low-level gaps:

```text
silence 0.000000s-1.706667s
silence 31.65s-32.55s
silence 58.62s-60.28s
silence 63.27s-64.91s
```

Window and stereo metrics:

```text
first 100 ms window above -20 dBFS: 1.7s-1.8s
loudest 100 ms peak window: 26.9s-27.0s, peak 1.000000
loudest 100 ms RMS window: 43.7s-43.8s, -4.62 dBFS
centered L/R correlation: 0.998569
mid RMS:  -6.85 dBFS
side RMS: -28.78 dBFS
side vs mid: -21.93 dB
```

The artifact is strongly centered, with a small but present side component.

## Denormal Note

`ffmpeg astats` and the Python scan both found `75264` subnormal f32 samples per
channel. They occur as one contiguous run from `0.000000s` to `1.706667s`, the
same interval detected as startup silence.

This is not an audible failure in the artifact, but it is useful evidence for a
small follow-up: the master DC blocker should probably flush very small internal
state/output values to exact zero to avoid denormal tails on hosts that do not
flush subnormals automatically.

## Evidence Flow

The validation chain for this slice is:

```text
code change: master DC blocker in engine output
  -> tests: mamut-dsp, mamut-engine, mamut-standalone passed
  -> rebuild: debug and release mamut-standalone targets refreshed
  -> run: direct output capture from the rebuilt binary
  -> artifact: mamut-output-20260430-125653.wav
  -> analysis: DC mean moved from about +0.176 to about -0.0001 full-scale
```

This closes the original DC-offset finding for the direct-output path.

## Verdict

Pass for the DC blocker:

- direct f32 WAV recorder still works
- artifact duration and format are correct
- DC offset is reduced by roughly three orders of magnitude
- active material after startup is effectively centered

Follow-up required for audio quality:

- the current take hits the positive output clamp
- integrated loudness is very high for raw capture (`-4.0 LUFS`)
- true peak is above full scale (`+0.3 dBFS`)
- startup silence contains subnormal f32 samples, likely from a decaying filter
  state and worth flushing to exact zero
