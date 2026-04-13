# EPM1 Factory Bank Listening Checklist

Use this checklist when tuning the shipped `EPM1` factory bank. Each patch
should prove one clear role in the current product story.

## Locked Patch Roles

- `Molten Horizon`
  - role: best first demo patch
  - prove: wide `Horizont` body that can still lean into late rupture
  - listen for: `Bloom`, then `Gravitacija`, then `Ruin`
- `Cathedral Bloom`
  - role: `Horizont` pad
  - prove: stable air, width, and elegance without going thin
  - listen for: `Bloom` and `Swarm`
- `Ember Vault`
  - role: `Pec` bass
  - prove: dry furnace core and controlled low-mid force
  - listen for: `Heat` and `Gravitacija`
- `Razor Thaw`
  - role: `Baklja`-ready lead and edge-of-collapse voice
  - prove: playable strain before collapse
  - listen for: `Ruin`, aftertouch, and high `Gravitacija`
- `Gravity Wake`
  - role: `Gravitacija` performance patch
  - prove: clear inward-pull arc across a held phrase
  - listen for: `Gravitacija`, aftertouch, and mod wheel
- `Furnace Choir`
  - role: dense poly body
  - prove: multi-note weight without losing note center
  - listen for: chord repeats, sustain, and `Heat`
- `Granite Plain`
  - role: dry poly anchor
  - prove: FX-bypassed tone still feels authored
  - listen for: dry output, release shape, and low macro settings
- `Glass Tide`
  - role: wide animated patch
  - prove: motion from `Bloom` and `Swarm` without a hollow center
  - listen for: chorus/reverb support, stereo width, and long holds

## Common Failure Signs

- `Bloom` only sounds brighter instead of larger
- `Heat` turns muddy before it feels heavy
- `Ruin` jumps straight to fizz instead of steerable strain
- `Gravitacija` feels like a gain or filter shortcut instead of inward pull
- dry patches collapse when FX are bypassed
- sustain or release makes the allocator obvious
- animated patches lose the center and stop reading as one instrument

## Session Pass

Run this minimum pass before calling the bank stable:

- play all eight factory patches from a MIDI controller
- repeat dense chords on `Furnace Choir` and `Granite Plain`
- sweep all five macros on `Molten Horizon` and `Gravity Wake`
- use aftertouch on `Gravity Wake` and `Razor Thaw`
- compare `Glass Tide` with FX enabled and disabled
- verify `Ember Vault` stays dry and useful at low buffer sizes

