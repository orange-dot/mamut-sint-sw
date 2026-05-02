# Gravitacijska Fazna Mreza (GFM)

DSP-level istrazivacka ideja za sledecu generaciju mamut-sint-sw voice engine-a.

- Datum: 2026-04-29
- Status: brainstorm / istrazivacka grana — capture, ne plan, ne decision
- Scope type: radical alternate voice engine, not a postprocessing stage
- Scope: workspace/systems/mamut-sint-sw-remote-up-21-4
- Acronym: `GFM`

The old short name is intentionally retired because it collides with license
language in this repository. `GFM` means only `Gravitacijska Fazna Mreza`.

Related research index:

- `next-generation-dsp-research-tracks.md`
- `implementation-language-strategy.md`
- `material-core-idea.md`

Related reference runtime:

- `../../../../../forks/systems/ekk-runtime-main/README.md`
- `../../../../../forks/systems/ekk-runtime-main/include/ekk/ekk_topology.h`
- `../../../../../forks/systems/ekk-runtime-main/include/ekk/ekk_field.h`
- `../../../../../forks/systems/ekk-runtime-main/include/ekk/ekk_module.h`
- `../../../../../forks/systems/ekk-runtime-main/include/ekk/ekk_consensus.h`

---

## Kontekst

Trenutni mamut-sint-sw je solidan VA-style subtractive synth:

- `mamut-dsp` nosi DSP primitives: oscilatori, ADSR, linear smoother, SVF,
  chorus/reverb buffers.
- `mamut-identity` prevodi makroe (`gravitacija`, `bloom`, `heat`, `ruin`,
  `swarm`) u identitete (`Horizont`, `Pec`, `Baklja`) i izvedena stanja:
  `mass`, `strain`, `headroom`, `body_focus`, `spatial_dispersion`. Vidi
  `mamut-identity/src/lib.rs:88`.
- `mamut-engine` orkestrira voice graph: cutoff, width, crossfeed, sync,
  crossmod, resonance, drive, plus character stage: mid/side, body memory,
  saturation, low-mid emphasis, side reduction/focus, crossfeed.

Inovacija je danas u **kontrolnom/identitetskom sloju**, ne u DSP primitivama.
DSP primitives su standardne: `mamut-dsp/Cargo.toml:8` ima prazan
`[dependencies]`, znaci ni fundsp ni rodio, raw Rust. Ovaj dokument predlaze
kako bi izgledalo kad bi DSP primitiva sama bila dovoljno radikalna da pokrije
semanticku ambiciju "gravitacija / pec / baklja" doslovce: kao polje, ne kao
naming convention preko VA modulatora.

## Centralna Ideja

Voice **nije** VA chain. Voice **jeste** 2D toroidalna mreza fazno-povezanih oscilatora.

Svaka celija `(i,j)` ima fazu `θ_ij(t)`. Dinamika:

```
dθ_ij/dt = ω_ij
         + Σ_neighbors K_ij,kl · sin(θ_kl − θ_ij)
         − γ_ij · |∇θ|²
         + ξ_ij(t)
```

Gde su:

- `ω_ij` — lokalna prirodna frekvencija (profilisana preko mreze)
- `K_ij,kl` — coupling matrica (8 neighbors, spatial)
- `γ_ij` — lokalno priguenje (moze biti negativno u ruin rezimu)
- `ξ_ij` — stohasticko forsiranje

Ovo nije Kuramoto kao academic toy. Ovo je *jezgro instrumenta*.

## EKK-Inspired Anti-Raspad Discipline

`GFM` should borrow stabilization patterns from `ekk-runtime`, but not its
distributed runtime code. The transferable idea is:

```text
radical local dynamics + bounded neighbor influence + decaying fields
+ health states + threshold gates = controlled swarm behavior
```

This is the anti-raspad layer. The phase lattice is allowed to become strange,
but it is not allowed to become unbounded, all-to-all, or numerically contagious.

What transfers:

- fixed-cardinality local influence
- decaying published fields
- gradient-following from neighbor aggregate to local state
- degraded-vs-hard-failure thinking
- threshold gates before structural state changes
- bounded arrays and deterministic tick ordering

What does not transfer:

- heartbeat messages
- distributed consensus protocol
- POSIX HAL
- shared-memory field region
- `ekk` C runtime ownership of audio semantics

### K=7 Cell Topology

Each lattice cell should have exactly `K=7` influence neighbors.

This is not an arbitrary magic number. In `GFM`, `K=7` is a bounded-influence
rule:

```text
cost = cells * 7
not
cost = cells * cells
```

The seven neighbors can be selected from a toroidal grid plus a small number of
identity-warped links. The topology should be recomputed only when the lattice
shape, identity regime, or macro field changes enough to matter, not on every
audio sample.

Suggested reference shape:

```rust
const GFM_K_NEIGHBORS: usize = 7;

struct GfmCellTopology {
    neighbors: [u16; GFM_K_NEIGHBORS],
    weights: [f32; GFM_K_NEIGHBORS],
}
```

`Swarm` should primarily affect this topology:

- more local clustering
- more heterogeneous weights
- more moving/nonuniform neighbor choice
- less global coherence, unless `Gravitacija` pulls it back into lock

### Decaying Cell Fields

Each cell should publish a small local field to its neighbors.

Candidate components:

```text
energy
strain
fracture
heat
coherence
headroom
```

Neighbors are sampled with temporal decay before gradients are computed:

```text
decayed_neighbor_field = neighbor_field * decay(age, tau)
aggregate = weighted_average(decayed_neighbor_fields)
gradient = aggregate - self_field
```

This gives swarm motion without a global conductor. It also prevents old energy
from locking the field forever.

The field is not a separate process or shared-memory service inside `Mamut`.
It is local DSP state inside `mamut-field`.

### Cell Health And Quarantine

The heartbeat idea becomes numeric health, not messaging.

```rust
enum GfmCellHealth {
    Healthy,
    Suspect,
    Quarantined,
    Recovering,
}
```

A cell becomes `Suspect` when it produces unsafe state:

- non-finite phase, field, or output
- energy above a configured ceiling
- strain above a configured ceiling
- fracture stuck high for too long
- phase velocity outside the allowed band

`Suspect` cells do not disappear. They are damped:

```text
coupling *= suspect_coupling_scale
output_weight *= suspect_output_scale
damping += suspect_damping
```

`Quarantined` cells contribute little or no probe output, but they keep a
recoverable state. The goal is degraded sound, not callback failure.

### Threshold Rupture Gates

`Baklja` should not mean that every hot cell ruptures independently. That would
turn the lattice into unstructured noise.

Rupture should require local agreement:

```text
self_ready = strain > rupture_threshold
neighbor_votes = count(K neighbors where strain/fracture is also high)
rupture_allowed = self_ready && neighbor_votes >= rupture_quorum
```

This borrows the threshold idea from `ekk-runtime` without importing its
distributed consensus protocol.

Optional inhibition:

```text
if rupture_wave starts in one neighborhood:
  temporarily raise rupture_threshold nearby
```

That allows dramatic local events without every adjacent cell rupturing at the
same time.

### Deterministic Step Order

The GFM update should follow one explicit order. This is the DSP equivalent of
`ekk_module_tick()` having one authoritative logical time.

Recommended step order:

1. Read current identity/direct parameters for this audio frame or slow tick.
2. Refresh topology only if the topology inputs changed.
3. Sample each cell's `K=7` neighbor fields with decay.
4. Compute gradients from neighbor aggregate to local field.
5. Update energy, strain, heat, coherence, and headroom.
6. Evaluate rupture thresholds and inhibition.
7. Update cell health and apply damping/quarantine rules.
8. Integrate phase.
9. Read probe output.
10. Sanitize output and update diagnostics.

This order should be documented in code comments and tests. A future C hot
kernel must preserve the same order exactly.

## Mapping: identitetski makroi -> field parametri

Identity macros nisu vise modulator routing. Oni su direktni parametri polja:

| Macro / state          | Field parameter                                            |
|------------------------|------------------------------------------------------------|
| gravitacija            | K (coupling — visok K ⇒ sync, kristalisanje, lock-in)      |
| bloom                  | spatial korelacija stohastickog ξ                          |
| heat (Pec)             | ω-disperzija + termalni noise                              |
| ruin                   | lokalna inverzija γ (energija pumpana u vrtloge)           |
| swarm                  | spatial heterogenost K (lokalno klasterovanje)             |
| mass                   | d²θ/dt² koeficijent (inertia, sporiji response)            |
| strain                 | asimetrija coupling matrice (preferirane fazne relacije)   |
| body_focus             | probe footprint (mali = ostro, veliki = zamucen read)      |
| spatial_dispersion     | multi-probe spread                                         |

## Identitetske posture

Identiteti (Horizont, Pec, Baklja) postaju **lattice rezimi**, ne preseti:

- **Horizont** — veliki lattice, spor ω-tilt preko x-ose, nisko K. Horizont se
  *cuje* kao frekvencijski gradijent.
- **Pec** — visoka ω-disperzija, jako stohasticko forsiranje, energetski gust,
  termalni rezim.
- **Baklja** — lokalna sjajna ekscitacija. Probe se krece brzo, ostavlja trag
  preko mreze.

## Probe model (kako voice postaje audio)

Audio izlaz = jedan ili vise probe-ova citaju lattice. Probe je sonda / gudalo /
igla na ploci.

- Pozicija probe = trenutni timbre (*gde* citas)
- MIDI velocity = dubina utiskivanja
- Aftertouch = lokalno zagrevanje pod sondom (lokalno raise ω, raise ξ)
- Mod wheel = global K ili macro selector
- Multi-probe = multi-finger choreography preko zive povrsine

## Sta GFM Subsume-uje

Jedna primitiva pokriva sve sto trenutni mamut-engine radi rasclanjeno:

- **Oscillator** — lattice ima sopstvene frekvencije.
- **Filter** — lattice je distribuirani rezonantni medijum.
- **Modulation** — cross-coupling je intrinsicno.
- **Chorus / unison** — multi-mode interferencija je direktno tu.
- **Body memory** — lattice ima fazni memory: prethodne note ostavljaju
  residual phase distributions. Ovo je *prava* legato kontinuiranost umesto
  crossfade-a.

Semanticki: "gravitacija / horizont / pec / baklja" su fizikalne metafore. Sa
GFM-om one *jesu* fizika, ne naming preko VA modulatora.

## Zvucni karakter (predvidjanje, jos nije mereno)

- **Sync rezim** (visok K): chord-like locking, kristalno, "akord se sam stvara".
- **Dekoherentni rezim** (nizak K, visok ξ): sumovit, dahuci, "staklo na staklu".
- **Termalni rezim** (visok ω-var): bell-like, neharmonijski, perkusivan.
- **Vrtlozni rezim** (ruin > γ): self-sustaining oscillations, "ziva mreza pod prstom".
- **Probe motion**: bowing-like — *gde* sviras vredno koliko *kad*.

Ovo nije ni piano-like ni analog-like. Instrument **povrsine**: gudalo preko
rezonantne ploce koja je tecna.

## Cost (honest reality)

Lattice 16×16 = 256 oscilatora. Per-sample ~256 sin + neighbor-sum. Na
48 kHz × 8 voice-ova → ~100M op/s. Cortex-A55 to moze, ali je tesno.

### Pragmaticno: dvostepeni model

- **Slow layer ~1–2 kHz**: lattice integracija; makroi su field parametri;
  sporo dinamicki.
- **Fast layer audio rate**: 2–4 modalna rezonatora po voice-u; mode params i
  ekscitacija citaju se iz lattice-neighborhood-a pod probe-om.

Lattice = "telo", modal layer = "glas". Cost padne ~10×.

### Implementacioni hooks

- `mamut-dsp/Cargo.toml:8` ima prazan `[dependencies]` — nema legacy DSP-a koji
  bi morao biti kompromitovan. Cist start.
- Novi modul ili crate (`mamut-field`, ili dodatak u `mamut-dsp`) bi nosio GFM primitive.
- `mamut-identity/src/lib.rs:88` vec pravi derived states. Ti states postaju
  field parametri umesto VA modulator targets. Mapping sloj je vec skoro tu.
- Render path u mamut-engine: voice graph emituje lattice-probe sample umesto VA
  voice. Postojeci character stage (mid/side, body memory, saturation) moze
  ostati kao postprocessing posle voice-summing-a, ili otkriti da je polovina
  toga sad redundantna.
- Hot-path discipline: const-generic lattice size, fixed-size buffers, zero
  alloc na callback, SIMD-friendly inner loop. Vidi `dsp-rust-hot-path` skill
  za pravila.

### Language posture

GFM is a `Rust + C` line:

- Rust owns the reference model, identity mapping, tests, offline rendering, and
  engine integration.
- C is allowed only as a narrow hot kernel after profiling proves a real budget
  problem.
- Python, Julia, C++, Zig, and Faust are not part of the required base workflow.

## Susedni istrazivacki putevi

Krace, za potpunost — ne kao alternative GFM-u nego kao paralelne grane.

### 1. Bifurkacijsko-koordinatna sinteza

Voice = parametrizovani 3D ODE (custom familija, ne Lorenz off-the-shelf).
Macros su koordinate u bifurkacionom prostoru. `ruin` gura sistem iz limit
cycle u haos preko period-doubling kaskade. Bifurkacioni dijagram = mapa
tembrova.

Manje radikalno, jeftinije, manje semanticki bogato. Mogla bi biti prva prototipska grana ako GFM deluje preteran.

### 2. Ergodicki instrument

Jedan deterministicki tok `φ_t` na fazi `M`; identiteti su razlicite
*measurement* funkcije `f : M → ℝ`. Audio = `f(φ_t(x_0))`.

Isti "duh", razlicita "tela" — Horizont, Pec, Baklja su tri readout-a istog
flow-a. Vise art-installation nego playable, ali kao istrazivacka grana
zanimljiva.

## Preporuka

**GFM je centralni radikalni put.** Radikalno menja sta voice JESTE, a pri tome
se pravilno spaja sa postojecim semantickim slojem. Bez GFM-a, mamut je dobar
VA + identity engine. Sa GFM-om, mamut postaje instrument koji *fizicki ne moze
postojati izvan ovog koda*: drugaciji od svega na trzistu, i poseban tacno na
nivou koji je trazen: algoritamskom.

## Moguci sledeci koraci (ne commit, ne plan)

1. Prototip GFM primitive u zasebnom playground-u: Rust binary, dump WAV, plot
   lattice state.
2. Verifikovati zvucne rezime: da li se sync / decoherent / thermal / vortex
   rezimi cuju kako se ocekuje.
3. Mapirati sve makroe u field parametre, verifikovati da semantika
   "gravitacija = K" stvarno daje muzicki ocekivan rezultat.
4. Dvostepeni model: probati slow-layer-only verziju da se vidi koliko cost
   moze da padne pre nego sto izgubimo karakter.
5. Tek onda integracija u mamut-engine voice graph **kao alternativa, ne
   replacement**, dok se ne dokaze.

---

*Idea originated 2026-04-29. Capture, ne plan, ne decision.*
