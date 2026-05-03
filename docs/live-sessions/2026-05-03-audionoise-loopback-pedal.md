# 2026-05-03 AudioNoise ALSA Loopback Pedal Test

This note records the first local Mamut synth run through the `orange-dot/AudioNoise`
ALSA live pedal sidecar. The goal was to validate the external-pedal chain, not
to change Mamut code.

## Repositories

- Mamut repo: `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4`
- Mamut local HEAD before this note: `da8ee70 Restyle Sound Lab with PC4 controls`
- AudioNoise repo: `/home/dev/work-base-20260421/workspace/systems/AudioNoise`
- AudioNoise HEAD: `5dbd73c Merge pull request #2 from orange-dot/audionoise-live-alsa`

## Hardware And ALSA Map

Relevant devices observed:

```text
Yamaha AG06/AG03: hw:1,0
Loopback card:    hw:5
mioXM DIN 1:      16:0
```

The tested chain was:

```text
PC4 -> mioXM DIN 1 -> Mamut standalone
Mamut playback -> hw:5,0
AudioNoise capture <- hw:5,1
AudioNoise playback -> hw:1,0 Yamaha AG06/AG03
```

`snd-aloop` was loaded manually before the loopback test. The Codex sandbox did
not expose `/dev/snd`, so ALSA runtime commands were run outside the sandbox.

## Probes

Direct Yamaha output worked:

```bash
speaker-test -D hw:1,0 -r 44100 -c 2 -F S32_LE -t sine -f 440 -l 1
```

The operator heard the sine output.

AudioNoise probe for the target loopback-to-Yamaha path succeeded:

```bash
cd /home/dev/work-base-20260421/workspace/systems/AudioNoise

./live_alsa \
  --probe \
  --input hw:5,1 \
  --output hw:1,0 \
  --rate 44100 \
  --channels 2 \
  --period 128 \
  --buffer 512
```

Observed negotiation:

```text
capture accepted: format=FLOAT_LE rate=44100 channels=2 period_size=128 buffer_size=512
playback accepted: format=S32_LE rate=44100 channels=2 period_size=128 buffer_size=512
probe OK
```

Loopback direction was also verified with `speaker-test` writing to `hw:5,0`
while AudioNoise bypass read from `hw:5,1` and played to the Yamaha. The
operator heard that signal.

## Mamut Runtime

Mamut command:

```bash
cd /home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4

cargo run --locked -p mamut-standalone -- play \
  --headless \
  --audio-device hw:5,0 \
  --alsa-period-frames 128 \
  --alsa-buffer-frames 512 \
  --alsa-start-threshold-frames 512 \
  --midi-device "mioXM DIN 1" \
  --midi-channel 1 \
  --controller-profile profiles/pc4-full.toml \
  molten-horizon
```

Startup evidence:

```text
audio: hw:5,0 (Loopback, Loopback PCM) @ 44100 Hz, 2 channels
alsa: selector=hw:5,0 period=128 buffer=512 start_threshold=512
mode: connected (mioXM:mioXM DIN 1 16:0)
midi channel filter: channel 1
controller profile: pc4-full (profiles/pc4-full.toml)
```

During the first active pass, Mamut reported incoming MIDI and rendered voices:

```text
voices: active=6
midi activity: messages=80
peak=0.751
transport: queued=384 target=512 write_hint=128 underrun_batches=2 underrun_frames=256 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

## AudioNoise Pedal Passes

Bypass command:

```bash
./live_alsa \
  --bypass \
  --input hw:5,1 \
  --output hw:1,0 \
  --rate 44100 \
  --channels 2 \
  --period 128 \
  --buffer 512
```

The operator heard Mamut through AudioNoise bypass. This confirmed the Mamut
playback, loopback handoff, AudioNoise capture/playback, and Yamaha output path.

`svfdrive` full-wet command:

```bash
./live_alsa \
  --effect svfdrive \
  --input hw:5,1 \
  --output hw:1,0 \
  --rate 44100 \
  --channels 2 \
  --period 128 \
  --buffer 512 \
  --pot0 0.45 --pot1 0.55 --pot2 0.45 --pot3 0.25
```

Observed AudioNoise description:

```text
Live svfdrive: cutoff=789 Hz resonance=0.55 drive=0.45 strain=0.25
```

The full-wet pass was effectively too quiet or too closed for the tested Mamut
material. Returning to bypass restored audible Mamut output.

`svfdrive` with dry blend worked:

```bash
./live_alsa \
  --effect svfdrive \
  --wet 0.25 \
  --input hw:5,1 \
  --output hw:1,0 \
  --rate 44100 \
  --channels 2 \
  --period 128 \
  --buffer 512 \
  --pot0 0.45 --pot1 0.55 --pot2 0.45 --pot3 0.25
```

The operator heard the signal and reported a clear difference from bypass.

`echo` worked and made an audible difference:

```bash
./live_alsa \
  --effect echo \
  --wet 0.35 \
  --input hw:5,1 \
  --output hw:1,0 \
  --rate 44100 \
  --channels 2 \
  --period 128 \
  --buffer 512 \
  --pot0 0.35 --pot1 0.45 --pot2 0.35 --pot3 0.30
```

Observed AudioNoise description:

```text
Live echo:  delay=350 ms lfo=1.4 ms feedback=0.3
```

Switching AudioNoise between bypass and echo produced a clearly audible
difference.

## Patch Switch

The Mamut runtime was switched without restarting the ALSA chain:

```text
patch sawyer-rezz
```

Observed patch status:

```text
loaded patch: Sawyer Rezz
patch: Sawyer Rezz - OB-X-style brassy poly growl inspired by the Rush Tom Sawyer synth color
tags: poly, lead, brass, rezz, performance
```

The operator heard Sawyer Rezz through bypass and then through the AudioNoise
echo pedal.

## Result

The external ALSA pedal architecture is validated for this local rig:

```text
Mamut -> snd-aloop -> AudioNoise live_alsa -> Yamaha AG06/AG03
```

Confirmed:

- no Mamut code changes were needed
- strict `hw:` loopback path worked at 44100 Hz stereo
- AudioNoise accepted `FLOAT_LE` capture from loopback and `S32_LE` playback to Yamaha
- Mamut MIDI input and rendering continued while AudioNoise effects were restarted
- bypass, `svfdrive --wet 0.25`, and `echo --wet 0.35` were audible

Limitations:

- `svfdrive` full-wet was not useful for the tested patch level/color
- `live_alsa` stereo remains mono-folded in this version
- the tested desktop path showed small playback xrun counts at startup and during restarts; capture xrun count stayed at zero in the observed AudioNoise reports
