# EPM1 GUI Design System

Status: governs the `egui` IA restructure under ADR 0004.

This document defines the visual and layout system for the new GUI. The
implementation lives in `crates/mamut-standalone/src/gui/` (`egui`).
Tokens are paradigm-independent — they survived the withdrawal of ADR
0002 — and apply directly to the `egui` Painter and style APIs used by
the implementation. Where the spec previously offered toolkit-specific
implementation notes, this version collapses to the `egui` side per
ADR 0004.

The aesthetic synthesis is hardware-instrument character (matte finish,
arc-indicated knobs, panel chrome) over digital-plugin precision
(monospaced value readouts, fine typography, clear focus indication) at
performance-instrument density (controls large enough to read at distance,
high contrast). Modular signal-flow visualization is reserved for the
`mamut-field` GFM workbench inside `SOUND` and is not used as a primary
layout idiom on other screens.

## 1. Palette

All tokens are dark-mode only. There is no light theme.

| Token | Hex | Use |
|---|---|---|
| `bg.panel` | `#0E1218` | screen background |
| `bg.surface` | `#161B22` | panel body fill |
| `bg.recessed` | `#0A0D11` | knob well, value display recess, scope background |
| `bg.header` | `#1C232C` | panel header strip |
| `fg.primary` | `#E8EBEF` | screen titles, primary headings |
| `fg.value` | `#C9D1DA` | numeric value text |
| `fg.label` | `#8B95A1` | control labels, axis labels |
| `fg.subtle` | `#5B6571` | tick marks, grid lines, separators |
| `accent.primary` | `#4DD0E1` | live-active, focused control, selected item |
| `accent.secondary` | `#FFB74D` | knob indicator arc/dot |
| `state.warning` | `#FFB300` | xrun > 0, controller dropped, take-recorder warning |
| `state.error` | `#E53935` | clip detected, audio device error, plugin host error |
| `state.live` | `#FFFFFF` | very-active state (note-on transient, control just touched) |

Live-active treatment is a `1 px` solid `accent.primary` border applied to
the focused control or panel. It never blinks. Transient events (note-on,
controller change) may apply a `200 ms` decay pulse to `state.live`, then
return to the resting state.

## 2. Typography

Typography is by family + size + weight. Fallback chain in parentheses.

| Token | Family | Size | Weight | Use |
|---|---|---|---|---|
| `type.display` | Inter (system sans) | 18 px | 500 | screen title in perform-rail |
| `type.heading` | Inter (system sans) | 14 px | 600 | panel header text |
| `type.label` | Inter (system sans) | 11 px | 400 | knob/slider labels |
| `type.value` | JetBrains Mono (system mono) | 13 px | 500 | numeric readouts |
| `type.trace` | JetBrains Mono (system mono) | 10 px | 400 | MIDI trace lines on `INSPECT` |

Numeric readouts are always monospace so values do not jitter horizontally
as digits change.

## 3. Knob geometry

There are two knob sizes. The `main` knob is the default; the `fine` knob
is reserved for dense sub-panels and tertiary controls.

| Token | Diameter | Arc stroke | Indicator | Center | Tick marks | Center-to-center spacing |
|---|---|---|---|---|---|---|
| `knob.main` | 56 px | 1.5 px | 5 px dot | 2 px dot | at 0/25/50/75/100 %, 5 px outside arc | 84 px |
| `knob.fine` | 32 px | 1 px | 3 px dot | 1 px dot | none | 48 px |

The knob arc spans 270°, from 7:30 to 4:30 clock. The unlit arc segment is
drawn in `fg.subtle`; the lit segment (from 7:30 to the current value) is
drawn in `accent.secondary`. The indicator dot rides the end of the lit
segment. The center dot is always `accent.primary` when the knob is the
focused control, otherwise `fg.subtle`.

Numeric value text lives below the knob, baseline-aligned across a row of
knobs. The label text lives above the knob.

## 4. Panel chrome

Every panel is a rectangle with:

- `1 px` solid `bg.recessed` border
- `4 px` corner radius
- `28 px` tall header strip filled with `bg.header`, panel title in
  `type.heading` aligned left, optional toggle or menu icon aligned right
- `12 px` to `16 px` interior padding
- `bg.surface` body fill

Section panels on `SOUND` use the same chrome. Sub-panels inside a
section panel reduce the border to `1 px` `fg.subtle` and drop the corner
radius to `2 px`, to make hierarchy visible at a glance.

## 5. Density and rhythm

- Vertical baseline grid: `8 px`. All vertical spacing is a multiple of
  `8 px`.
- Knob spacing: at least `1.5 × diameter` center-to-center (84 px for
  `knob.main`, 48 px for `knob.fine`).
- Section panel margin: `16 px` between adjacent panels.
- Perform-rail height: `64 px` (two 28-px rows + 8 px padding).
- Tab/screen-switch bar height: `32 px`.

Density rule of thumb: a section panel should fit at least four `knob.main`
controls horizontally on the canonical `1280 × 720` window. If a panel
needs more controls, it splits into sub-panels rather than scrolling.

## 6. Live-active visual treatment

- Focused control: `1 px` solid `accent.primary` outline at the control's
  bounding box, no animation.
- Live-active panel (e.g. take recorder during a take): same `1 px`
  `accent.primary` outline at the panel bounding box.
- Transient event pulse (note-on, controller hit): `1 px` outline at
  `state.live`, decaying linearly to the resting outline color over
  `200 ms`. Pulses do not stack — a new event resets the decay.
- Never use blinking. Never use flashing red/green status indicators.

## 7. Screen layouts

The screen layouts below are wireframes, not pixel-exact. Box-drawing
characters are illustrative.

### 7.1 Perform-rail (always visible)

```
┌─ perform-rail ────────────────────────────────────────────────────────────┐
│ ▸ molten-horizon  slot 0  │ AG03 hw:1,0 96 kHz │ mioXM ch1 ● │ xrun 0     │
│ voices 4  sustain  clip   │ last: K3 macro/heat = 0.62  accepted          │
│ [panic]  [reset ctrls]  [quit]                                            │
└───────────────────────────────────────────────────────────────────────────┘
```

The action buttons (`panic`, `reset ctrls`, `quit`) are aligned to the
right edge in production; the diagram aligns left for clarity.

### 7.2 PERFORM

```
┌─ perform-rail ────────────────────────────────────────────────────────────┐
│ ... (always visible) ...                                                  │
├───────────────────────────────────────────────────────────────────────────┤
│  [PERFORM]   SOUND   SYSTEM   INSPECT                                     │
├───────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  ┌─ macros ──────────────────────────────────────────────────────────┐    │
│  │   ◯         ◯         ◯         ◯         ◯                        │    │
│  │ gravitacija bloom    heat      ruin     swarm                      │    │
│  │   0.42      0.18     0.62      0.00     0.31                       │    │
│  └────────────────────────────────────────────────────────────────────┘    │
│                                                                           │
│  ┌─ live set ──────────────────┐  ┌─ factory bank ─────────────────┐      │
│  │ [0:molten-horizon] [1:cath.]│  │ molten-horizon       drone     │      │
│  │ [2:ember-vault   ] [3:razor]│  │ cathedral-bloom      pad       │      │
│  │ [4:gravity-wake  ] [5:furn.]│  │ ember-vault          bass      │      │
│  │ [6:granite-plain ] [7:glass]│  │ ...                            │      │
│  └─────────────────────────────┘  └────────────────────────────────┘      │
│                                                                           │
│  ┌─ activity ─────────────────────────────────────────────────────────┐   │
│  │ held: 36 40 43      scope ▁▂▃▅▆▇█▆▅▃▂▁                              │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│  ┌─ take recorder ────────────────────────────────────────────────────┐   │
│  │ [▶ record] [■ stop]   60 s   tag: ___   state: idle                │   │
│  └────────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────────────┘
```

Optional sidebars (collapsed by default) on `PERFORM`:

- Read-only `PC4` controller mirror — knob/slider/switch state from the
  connected hardware controller. Collapsed default; expand-on-click reveals
  the binding map.
- Read-only `GFM` field mirror — small scalar field thumbnail. Collapsed
  default; the edit-capable workbench lives on `SOUND`.

### 7.3 SOUND (high-level)

```
┌─ perform-rail ────────────────────────────────────────────────────────────┐
├─ PERFORM   [SOUND]   SYSTEM   INSPECT ────────────────────────────────────┤
│ ┌── sections ──┐ ┌── focused section ──────────────────────────────────┐  │
│ │ ◉ Osc          │ │ ┌─ osc 1 ────────┐ ┌─ osc 2 ────────┐              │  │
│ │ ○ Filter       │ │ │ ◯ wave ◯ tune  │ │ ◯ wave ◯ tune  │              │  │
│ │ ○ Body         │ │ │ ◯ mix  ◯ fine  │ │ ◯ mix  ◯ fine  │              │  │
│ │ ○ Spectral     │ │ └────────────────┘ └────────────────┘              │  │
│ │ ○ Relations    │ │ ┌─ noise ────────┐ ┌─ spectral ─────┐              │  │
│ │ ○ Masnoca      │ │ │ ◯ amt  ◯ color │ │ ◯ tilt ◯ scale │              │  │
│ │ ○ Motion       │ │ └────────────────┘ └────────────────┘              │  │
│ │ ○ GFM Field    │ │                                                    │  │
│ │ ○ BCS Layer    │ └────────────────────────────────────────────────────┘  │
│ │ ○ Master       │                                                          │
│ └────────────────┘ MIDI focus: K2 → osc.1.mix (CC 17, value 0.42)          │
└───────────────────────────────────────────────────────────────────────────┘
```

Section list is the left rail. The right pane fills with knob-heavy
sub-panels for the focused section. `GFM Field` and `BCS Layer` are
sections like the rest. Switching sections changes only the right pane.

### 7.4 SYSTEM (high-level)

```
┌─ perform-rail ────────────────────────────────────────────────────────────┐
├─ PERFORM   SOUND   [SYSTEM]   INSPECT ────────────────────────────────────┤
│ ┌─ audio ───────────┐ ┌─ midi ─────────────┐ ┌─ recorder ────────────┐    │
│ │ device  hw:1,0    │ │ port    mioXM DIN 1│ │ dir     captures/     │    │
│ │ rate    96 kHz    │ │ channel 1          │ │ format  f32 WAV       │    │
│ │ period  512       │ │ profile pc4-full   │ │ default 60 s          │    │
│ │ buffer  2048      │ │ ...                │ │ ...                   │    │
│ │ start   2048      │ │                    │ │                       │    │
│ └───────────────────┘ └────────────────────┘ └───────────────────────┘    │
│                                                                           │
│ ┌─ pc4 profile bindings ──────────────────────────────────────────────┐   │
│ │ K1 → bloom (CC 16)    K2 → heat (CC 17)    K3 → ruin (CC 18)  ...   │   │
│ └─────────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│ ┌─ recovery ──────────────────────────────────────────────────────────┐   │
│ │ [reset transport]  [reload patch]  [restart audio]                  │   │
│ └─────────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────────────┘
```

### 7.5 INSPECT (high-level)

```
┌─ perform-rail ────────────────────────────────────────────────────────────┐
├─ PERFORM   SOUND   SYSTEM   [INSPECT] ────────────────────────────────────┤
│ ┌─ voices ────────────────────────┐ ┌─ midi trace ──────────────────┐     │
│ │ #0  note 36  vel .42  released  │ │ +0ms   CC 16 = 0.42  accepted │     │
│ │ #1  note 40  vel .31  held      │ │ +12ms  CC 17 = 0.18  accepted │     │
│ │ #2  note 43  vel .55  held      │ │ +28ms  note 36 vel 92         │     │
│ │ #3  -                           │ │ ...                           │     │
│ └─────────────────────────────────┘ └───────────────────────────────┘     │
│                                                                           │
│ ┌─ counters ──────────────────────────────────────────────────────────┐   │
│ │ midi  12428 / accept 12380 / drop 0 / coalesced 312                 │   │
│ │ xruns 0 / underrun 0 / overflow 0 / queued 2048                     │   │
│ │ rec   frames 0 / dropped 0                                          │   │
│ └─────────────────────────────────────────────────────────────────────┘   │
│                                                                           │
│ ┌─ identity ────────┐ ┌─ derived ─────────┐ ┌─ patch dump ────────────┐   │
│ │ horizont_open 0.42│ │ mass    0.18      │ │ {                       │   │
│ │ pec_mass     0.18 │ │ strain  0.62      │ │   "name": "molten...",  │   │
│ │ baklja_ready 0.62 │ │ headroom 0.71     │ │   "macros": { ... },    │   │
│ │ ...               │ │ ...               │ │   ...                   │   │
│ └───────────────────┘ └───────────────────┘ └─────────────────────────┘   │
│                                                                           │
│ ┌─ master oscilloscope (full) ────────────────────────────────────────┐   │
│ │ ▁▂▄▆█▇▆▅▃▂▁▁▂▃▄▅▆▇█▇▅▃▂▁                                            │   │
│ └─────────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────────────┘
```

## 8. Window canonical size

The canonical PERFORM-screen window is `1280 × 720`. The layout must
work down to `1024 × 640` and up to `1920 × 1080` without breaking the
cut. Section panels are responsive within ±20 %; if a panel would shrink
below readability, an overflow indicator (`...`) is shown instead of
crushing the controls.

## 9. High-risk widgets

### 9.1 GFM lattice paint

The `mamut-field` GFM workbench renders a 2D scalar field. The
implementation uses `egui::Painter` to draw each lattice cell as a
small rectangle. For a `32 × 32` lattice (1024 cells), this is well
within `egui`'s rasterization budget at 60 fps.

Build the cell rect list once per snapshot tick (every 75 ms per
`PERFORMANCE_UI_REFRESH` in
`crates/mamut-runtime/src/types/constants.rs`); redraw every frame.
Color is sampled from `accent.secondary` (high amplitude) through
`fg.label` (mid) to `bg.recessed` (low). Estimated cost: `~1 ms` per
frame at `32 × 32`.

If the lattice grows past `64 × 64` or the `egui::Painter` path misses
the 30 fps floor, evaluate `egui-wgpu` paint callbacks for a
shader-based approach (fullscreen quad in the workbench panel,
fragment shader sampling an `R32F` texture uploaded each snapshot
tick). That decision is deferred until the workbench lands on `SOUND`
in the IA restructure.

### 9.2 Master oscilloscope

8192-frame ring buffer fed by the audio thread. At a 60-fps display, the
scope downsamples to ~1024 sample points per frame via peak-pick (max,
min) over each 8-sample window. The visible polyline is a single
`egui::Painter::add(Shape::line(...))` stroked in `accent.primary`.
Sticky scope on `PERFORM` is `~150 px` tall; the full scope on `INSPECT`
is sized to fit the available panel.

### 9.3 Rotary knob

Custom `egui::Painter`-based widget in
`crates/mamut-standalone/src/gui/widgets/`. Geometry per section 3.
Mouse drag is vertical: `1 px` = `0.5 %` of full range by default, with
`shift` modifier reducing sensitivity to `0.1 %` and `ctrl` modifier
reducing to `0.02 %`. Scroll wheel is `1 notch` = `1 %`. Double-click
opens an inline numeric entry. Right-click resets to the patch default.

Knob taper (the mapping from `0..1` knob position to parameter range) is
linear by default. Frequency-domain parameters (filter cutoffs,
oscillator pitch) and amplitude-in-dB parameters use a logarithmic
taper. The per-parameter taper is inherited from `mamut-params` and is
not redefined here; the knob view reads the parameter's declared taper
from the parameter registry rather than carrying its own taper table.

## 10. Implementation notes

Token constants live in `crates/mamut-standalone/src/gui/style.rs`:

- Palette tokens are `egui::Color32` constants reached via small
  accessor functions (`style::accent_primary()`, `style::bg_panel()`,
  etc.) so call sites never reach for `egui::Color32::from_rgb(...)`
  literals.
- Typography tokens are `egui::TextStyle` overrides on a custom
  `egui::Style` applied at app construction.
- Dimension constants (`KNOB_MAIN = 56.0`, `KNOB_FINE = 32.0`,
  perform-rail height, panel padding) are bare `f32` constants used at
  paint sites.

Custom widgets — knob, oscilloscope, GFM lattice, perform-rail,
slot-grid, macro-meter, perform-badges — live in
`crates/mamut-standalone/src/gui/widgets/`. Each widget is dumb: it
takes data + closures + style accessors, returns an `egui::Response`,
and never reaches for a `RuntimeSession` reference. Write surfaces
(`panic`, `reset_controllers`, `set_macro`, `set_direct_param`) flow
through `SessionCommandSink` per ADR 0004 doctrine; widgets do not
hold the sink directly — the caller threads it through closures.

## 11. What this spec deliberately does not decide

- Specific font files. The system fonts are acceptable for the
  bring-up; vendored fonts are a later polish concern.
- Animation timing beyond the `200 ms` transient pulse.
- Locale or i18n; the GUI is English-only for `EPM1`.
- Touch input; the GUI is mouse + keyboard, and a separate touch
  controller path is anchored in `docs/android-touch-controller.md`.
- Window resize behavior beyond the `1024–1920` envelope stated in
  section 8.
- Light theme. There is no light theme.
