# EPM1 RPi3B mioXM Headless Runbook

This is the v1 Raspberry Pi Linux path for `EPM1`.

Target posture:

- board: Raspberry Pi 3B
- OS: Raspberry Pi OS Lite 64-bit
- MIDI: `PC4 -> mioXM -> Raspberry Pi USB-A`
- audio: Raspberry Pi analog jack through ALSA `hw:<card>,<device>`
- runtime: `mamut-standalone` without the `gui` feature

This is a first hardware smoke path. The Pi analog jack is not the final audio
quality target.

## Host Build

From the `mamut-sint-sw` repo root:

```sh
tools/build-rpi3b-headless.sh
```

The default target is `aarch64-unknown-linux-gnu` and the binary is built with:

```text
--no-default-features
```

That keeps `eframe`/GUI dependencies out of the Pi binary. The resulting binary
supports `play --headless`; `play --gui` is intentionally unavailable in this
build.

Expected host prerequisites for the default target:

```sh
rustup target add aarch64-unknown-linux-gnu
```

Install or expose an `aarch64-linux-gnu` linker and arm64 ALSA development files
through the local cross-build environment. If the local cross setup uses a
different linker, set:

```sh
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=/path/to/aarch64-linux-gnu-gcc
```

## Deploy

Deploy the binary, factory patches, controller profiles, and Pi run script:

```sh
tools/deploy-rpi3b-headless.sh pi@raspberrypi.local
```

Defaults:

- remote directory: `/home/pi/mamut-epm1`
- binary: `target/aarch64-unknown-linux-gnu/release/mamut-standalone`

Override when needed:

```sh
MAMUT_PI_HOST=pi@192.168.1.42 \
MAMUT_PI_DEPLOY_DIR=/home/pi/mamut-epm1 \
tools/deploy-rpi3b-headless.sh
```

## Pi Preflight

On the Pi:

```sh
sudo apt install alsa-utils libasound2
cd /home/pi/mamut-epm1
./run-rpi3b-mioxm.sh --list
```

Expected:

- `list-audio` shows the Pi analog/Headphones playback device
- `list-midi` shows a mioXM input port

If analog audio is not listed, enable the Pi headphones/analog output in
`raspi-config` and confirm that `/proc/asound/cards` contains a headphones or
`bcm2835` playback card.

If detection chooses the wrong output, pass the ALSA selector explicitly:

```sh
MAMUT_RPI_AUDIO_DEVICE=hw:<card>,<device> ./run-rpi3b-mioxm.sh --list
```

## First Smoke

Use terminal mode for the first pass so `status`, `panic`, and `quit` are
available over SSH. This is still no-GUI.

```sh
cd /home/pi/mamut-epm1
./run-rpi3b-mioxm.sh --tui --trace-midi molten-horizon
```

Defaults:

- patch: `molten-horizon`
- MIDI selector: `mioXM DIN 1`
- MIDI channel: `2`
- controller profile: `profiles/pc4-full.toml`
- sample rate: `48000`
- ALSA period/buffer/start threshold: `512 / 2048 / 2048`

Play from the PC4 and run:

```text
status
```

Acceptance for v1:

- notes from PC4 produce sound on the Pi analog jack
- MIDI accepted count rises
- `midi_dropped`, `runtime_dropped`, and `trace_dropped` stay at `0` during
  normal playing
- no sustained growth in underruns or xrun recoveries during a five-minute
  basic pass
- `panic` clears held notes/controller residue

For unattended/no-controls launch after the diagnostic pass:

```sh
./run-rpi3b-mioxm.sh molten-horizon
```

## Overrides

Common useful overrides:

```sh
MAMUT_RPI_AUDIO_DEVICE=hw:0,0 ./run-rpi3b-mioxm.sh --tui
MAMUT_MIDI_DEVICE=1 ./run-rpi3b-mioxm.sh --tui
MAMUT_SAMPLE_RATE=44100 ./run-rpi3b-mioxm.sh --tui
MAMUT_ALSA_PERIOD_FRAMES=1024 MAMUT_ALSA_BUFFER_FRAMES=4096 ./run-rpi3b-mioxm.sh --tui
```

Use `--demo` to separate audio output from the mioXM/PC4 path:

```sh
./run-rpi3b-mioxm.sh --tui --demo molten-horizon
```

## Failure Triage

- No audio device: run `./run-rpi3b-mioxm.sh --list`, enable headphones output,
  or pass `MAMUT_RPI_AUDIO_DEVICE=hw:<card>,<device>`.
- ALSA rate error: retry with `MAMUT_SAMPLE_RATE=44100`.
- No MIDI device: check USB cable, mioXM power/routing, then use a numeric
  `MAMUT_MIDI_DEVICE` from `list-midi`.
- Ambiguous MIDI selector: use the numeric `list-midi` index instead of
  `mioXM DIN 1`.
- Clicks or rising underruns: increase ALSA period/buffer first; do not add
  systemd/autostart until the SSH diagnostic pass is clean.
