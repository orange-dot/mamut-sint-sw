# 2026-04-29 PC4 AG03 Full Controller Map Session

## Boundary

This run validated the one-way hardware control path for the full PC4 profile:

- controller: Kurzweil `PC4`
- PC4 mode: Performance
- PC4 Global MIDI posture: clean idle setup, `Program Change` left `Off`
- MIDI path: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- audio path: `Mamut EPM1 -> Yamaha AG06/AG03`
- runtime mode: standalone headless with MIDI trace
- launch patch: `molten-horizon`

Canonical command:

```bash
tools/run-pc4-ag03.sh --audio-device hw:1,0 --trace-midi molten-horizon
```

## Cold Idle

Startup was clean after the PC4 Global MIDI cleanup:

```text
midi activity: messages=0
voices: active=0 sustain=false held=[]
transport: queued=512 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

No startup note spam, CC spam, or program-change switch was observed.

## Sustain

The sustain pedal produced clean down/up CC64 events:

```text
midi trace: ch=1 raw=[B0 40 7F] sustain down=true
midi trace: ch=1 raw=[B0 40 00] sustain down=false
```

## Knobs

The full K1-K9 surface was observed on channel 1 and matched
`profiles/pc4-full.toml`.

```text
K1 CC71 -> Gravitacija macro
K2 CC73 -> Bloom macro
K3 CC72 -> Amp Attack direct param
K4 CC79 -> Amp Release direct param
K5 CC14 -> Swarm macro
K6 CC17 -> Heat macro
K7 CC18 -> Ruin macro
K8 CC3  -> reserved, no synth action
K9 CC9  -> Reverb Mix direct param
```

K8 was useful as a detection aid: MIDI arrived and was explicitly classified as
reserved.

## Sliders

The full S1-S9 surface was observed on channel 1 and matched
`profiles/pc4-full.toml`.

```text
S1 CC12 -> Osc1 Saw Level
S2 CC13 -> Osc1 Pulse Level
S3 CC22 -> Osc2 Saw Level
S4 CC23 -> Sub Level
S5 CC24 -> Pre-Filter Drive
S6 CC25 -> Body Mix
S7 CC26 -> Filter Cutoff
S8 CC27 -> Chorus Mix
S9 CC28 -> Final Stage Body Drive
```

Representative log evidence:

```text
midi trace: ch=1 raw=[B0 1A 67] profile S7 direct param Filter Cutoff value=0.811 -> direct param Filter Cutoff value=5421.265
midi trace: ch=1 raw=[B0 1B 70] profile S8 direct param Chorus Mix value=0.882 -> direct param Chorus Mix value=0.882
midi trace: ch=1 raw=[B0 1C 70] profile S9 direct param Final Stage Body Drive value=0.882 -> direct param Final Stage Body Drive value=0.882
```

## Switches

The full SW surface was observed except release events where the runtime
correctly ignores release-side values for trigger actions.

```text
SW1 CC80 -> panic
SW2 CC81 -> reset controllers
SW3 CC82 -> previous favorite
SW4 CC83 -> next favorite
SW5 CC85 -> toggle Chorus Enabled
SW6 CC86 -> toggle Reverb Enabled
SW7 CC87 -> favorite slot 0
SW8 CC89 -> favorite slot 4
SW9 CC90 -> reserved
```

Representative log evidence:

```text
midi trace: ch=1 raw=[B0 55 7F] profile SW5 toggle Chorus Enabled value=1.000 -> toggle Chorus Enabled
midi trace: ch=1 raw=[B0 56 7F] profile SW6 toggle Reverb Enabled value=1.000 -> toggle Reverb Enabled
midi trace: ch=1 raw=[B0 5A 7F] profile SW9 reserved value=1.000
midi trace: ch=1 raw=[B0 50 7F] profile SW1 panic value=1.000 -> panic
midi trace: ch=1 raw=[B0 51 7F] profile SW2 reset controllers value=1.000 -> reset controllers
midi trace: ch=1 raw=[B0 52 7F] profile SW3 previous favorite value=1.000 -> previous favorite
midi trace: ch=1 raw=[B0 53 7F] profile SW4 next favorite value=1.000 -> next favorite
midi trace: ch=1 raw=[B0 57 7F] profile SW7 favorite slot=0 value=1.000 -> favorite slot=0
midi trace: ch=1 raw=[B0 59 7F] profile SW8 favorite slot=4 value=1.000 -> favorite slot=4
```

The live patch-switch path survived the SW3/SW4/SW7/SW8 actions with the
exclusive AG03 `hw:1,0` device open. No ALSA `EBUSY` or runtime crash was
observed.

Open note: the final `status` snapshot still reported `patch: Molten Horizon`
and `live slot: 0`, while the event log included `controller action -> live slot
4 (Gravity Wake)` and a `Gravity Wake` load. Treat this as a follow-up check for
status/log ordering or live-slot display consistency, not as a MIDI-map failure.

## Wheels And Aftertouch

The performance controls worked end to end:

```text
mod wheel: CC1, observed 0.000 -> 1.000 -> 0.000
pitch bend: observed -2.000 -> +2.000 semitones and return to 0.000
aftertouch: channel pressure D0, observed 0.000 -> 1.000 -> 0.000
```

Representative log evidence:

```text
midi trace: ch=1 raw=[B0 01 7F] mod wheel amount=1.000
midi trace: ch=1 raw=[E0 7F 7F] pitch bend semitones=2.000
midi trace: ch=1 raw=[E0 00 00] pitch bend semitones=-2.000
midi trace: ch=1 raw=[D0 7F] aftertouch pressure=1.000
```

## Program Change

Program Change was intentionally not tested in this run. The PC4 Global MIDI
setting remained `Program Change Off`, which is currently the preferred stable
performance posture because it prevents startup or entry program changes from
switching Mamut patches unexpectedly.

The Mamut side should still keep Program Change support for deliberate live-slot
switching, but PC4-side re-enable/send/disable should be a separate focused
acceptance run.

## Final Status

The final operator status showed no active voices and no MIDI overflow:

```text
voices: active=0 sustain=false held=[] peak=0.032 clip=false
midi activity: messages=941
transport: queued=512 target=512 write_hint=256 underrun_batches=25 underrun_frames=6400 xrun_recoveries=1 overflow_batches=0 overflow_frames=0
macros: gravitacija=0.000 bloom=0.000 heat=0.000 ruin=0.000 swarm=0.000
```

Transport counters increased during the long controller sweep and live
patch-switch actions, including one xrun recovery. There was no MIDI overflow,
no stuck note, and no device-busy failure.

## Verdict

Pass:

- clean idle held with the current PC4 Global MIDI setup
- sustain pedal works
- K1-K9 map works, including K8 reserved detection
- S1-S9 map works
- SW1-SW9 map works, including reserved SW9 detection
- mod wheel, pitch bend, and channel aftertouch work
- live patch-switch actions survived on AG03 `hw:1,0`
- no stuck voices remained at session close

Not covered:

- deliberate Program Change slot sweep, because PC4 Global `Program Change`
  stayed `Off`

Follow-up:

- verify final `status` live-slot/patch reporting after SW8 slot 4 in a short
  focused run
- keep Program Change disabled by default on PC4 unless the test is explicitly
  about live-slot program changes
