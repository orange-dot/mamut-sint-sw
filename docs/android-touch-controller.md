# Android Touch Controller Direction

## Decision

Build the Mamut EPM Android touch controller as a native Android app in
Kotlin with Jetpack Compose.

The app is a live performance and sound-design surface for `EPM1`. It should
prioritize large touch targets, low friction during a live take, and clear
engine state over generic MIDI-controller flexibility.

## Why Kotlin And Compose

- Android's native MIDI surface is exposed through `android.media.midi`, which
  is directly available from Kotlin and Java.
- Jetpack Compose is a good fit for touch-first controls: faders, XY pads,
  tabbed engine pages, patch grids, toggles, and large emergency actions.
- Native Android keeps USB MIDI, Bluetooth MIDI, foreground-service behavior,
  device attach/detach, and permission handling under our control.
- Compose lets the UI grow into a Mamut-specific instrument surface instead of
  a generic remote-control grid.

Avoid starting with Flutter or React Native for this app. They can produce a
good-looking UI quickly, but USB MIDI, lifecycle handling, and live-control
latency are likely to become bridge/plugin work. This controller is close
enough to transport and device state that native Android is the conservative
path.

## Transport Strategy

Start with a network bridge between Android and the Mamut host.

The current lab state has `mioXM` connected to the Mamut host over USB DAW.
The Android device was visible to the host as RNDIS/networking, not as a USB
MIDI device. That makes a direct Android-to-mioXM USB DAW path unavailable
while the Mamut host is using the DAW port.

Preferred first transport:

- Android app sends Mamut-native control messages over USB tethering or LAN.
- Mamut host bridge forwards those messages into the runtime control path.
- Protocol carries `param_id + f32 value`, runtime actions, patch-slot actions,
  and optional page/focus metadata.

Fallback transport:

- Android app emits MIDI CC messages compatible with the existing PC4-style
  controller profile.
- This can target an external class-compliant MIDI/DIN interface into mioXM,
  or any route that presents Android as a real MIDI source.

Defer direct RTP-MIDI/mioXM routing until it is proven on the actual Android
device and mioXM preset. It is useful, but should not be the first dependency.

## UI Shape

Initial pages:

- `Live`: five public macros, patch slots, panic, reset, record controls, and
  compact health/peak feedback.
- `Engine`: touch pages matching the current Sound Lab model: Osc 1, Osc 2,
  Noise, Spectral, Relations, Body/Filter, Motion, Performance, and Layers.
- `XY`: performance pads for paired gestures such as
  `Gravitacija/Bloom`, `Heat/Ruin`, `Swarm/Filter`, and `Body/Drive`.
- `Actions`: large safe controls for panic, reset controllers, previous/next
  slot, recorder start/stop, and favorite slots.

The app should present Mamut concepts directly. It should not look like a
generic DAW mixer.

## Feedback Model

The first usable version can be send-only, but the architecture should assume
host feedback:

- active patch and live slot
- macro/direct parameter values
- MIDI/activity health
- peak/RMS and limiter telemetry
- optional oscilloscope/scope buffer later

Feedback is the main reason to prefer a Mamut-native bridge over pure MIDI CC.
MIDI CC is acceptable as fallback, but it is too narrow for a rich touch
surface.

## Open Questions

- Exact bridge protocol: WebSocket, UDP, or local TCP over USB tether/LAN.
- Whether Mamut host should expose the bridge inside `mamut-runtime` or as a
  small companion process.
- Whether the Android app should support both native bridge and MIDI output in
  the first release.
- How much scope/telemetry data is safe to stream during live audio without
  contaminating the hot path.

## First Implementation Slice

1. Define the Mamut touch-control message schema.
2. Add a host-side bridge that can submit runtime control events without
   changing the audio callback boundary.
3. Build the Kotlin/Compose app shell with Live, Engine, XY, and Actions pages.
4. Map the first control set: macros, patch slots, panic/reset, record, filter
   cutoff/resonance/drive/model, oscillator levels, and FX mix.
5. Add host feedback for active patch, current values, transport health, and
   peak.

Keep MIDI CC export as a compatibility mode, not as the primary product model.
