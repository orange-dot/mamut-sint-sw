# EPM1 pc4gen Consumer Smoke

This is the first live consumer check for the PC4-informed generator. It runs
`pc4gen live` as a virtual ALSA MIDI source, connects `mamut-standalone` to that
source, enables MIDI trace logging, and verifies that the EPM1 live profile sees
the expected performance controls.

```bash
tools/run-pc4gen-consumer-smoke.sh --audio-device hw:<card>,<device>
```

If the Yamaha AG06/AG03 playback device is visible in `/proc/asound`, the script
can usually detect it automatically:

```bash
tools/run-pc4gen-consumer-smoke.sh
```

The host must expose the ALSA sequencer at `/dev/snd/seq`; if it is missing,
enable/load `snd_seq` before running the live smoke.

This is a local integration smoke, not part of the default `cargo test --locked`
path.

## What It Proves

- Mamut can see the `pc4gen` ALSA sequencer source from `list-midi`.
- `mamut-standalone play --trace-midi` can open that source by name.
- The Mamut PC4 live profile receives notes, macro CCs, mod wheel, sustain,
  channel aftertouch, pitch bend, and program changes.
- The locked live-set program change slots `0..7` remain parseable.

The required `pc4gen` scenario is `mamut-epm1-smoke`. It emits `CC16..20`,
`CC1`, `CC64`, channel aftertouch, pitch bend, program change `0..7`, and a
short note sync after a startup pre-roll so Mamut has time to connect.

## Useful Overrides

```bash
PC4GEN_ROOT=/path/to/pc4gen \
PC4GEN_DERIVED=/path/to/synthetic-pc4-capture/derived \
MAMUT_AUDIO_DEVICE=hw:<card>,<device> \
MAMUT_MIDI_CHANNEL=1 \
tools/run-pc4gen-consumer-smoke.sh
```

Use `PC4GEN_PROFILE=/path/to/profile.json` to skip profile generation from the
derived capture tree.

The smoke writes logs to a temporary directory and prints that directory on
success. Keep the `mamut.stderr.log` file when debugging missing MIDI traces.
