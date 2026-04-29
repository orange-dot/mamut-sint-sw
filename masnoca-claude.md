# Masnoća, Toplina, Snaga: Deep Dive

## Kontekst

Ovaj dokument mapira vokabular projekta (`Horizont`/`Pec`/`Baklja`/`Gravitacija`) na
fiziku analognog zvuka, psihoakustiku, i konkretne digitalne implementacione
obaveze koje `mamut-sint-sw` (EPM1 software linija) mora da realizuje da bi
zvučao kao instrument a ne kao crtež analoga.

Izvorni referentni dokumenti:

- `../mamut-sint-hw/docs/sound-direction.md`
- `../mamut-sint-hw/docs/tonal-architecture.md`
- `../mamut-sint-hw/docs/oscillator-strategy.md`
- `../mamut-sint-hw/docs/dsp-subsystem-spec.md`
- `../mamut-sint-hw/docs/p1-discrete-vco-prototype.md`
- `../mamut-sint-hw/docs/analog-learning-roadmap.md`

## 1. Fizičko poreklo masnoće — šta je u analognom besplatno

### 1.1 Oscilator — ne postoji savršena visina

VCO koristi eksponencijalni konverter (matched BJT par, tempco otpornik). Tri
efekta rade za tebe:

- **Tempco drift**: `Vbe` tranzistora je ~−2 mV/°C. Kad se kutija zagreje,
  svaki glas drifta drugačije → nijedna dva glasa u polifoniji nisu u savršenom
  faznom odnosu. Ovo je izvor "ensemble" osećaja = `Horizont`.
- **Shot noise u ekspo paru** modulira trenutnu struju u timing kondenzator
  → pitch ima ~1–5 centi RMS "živog" šetanja na vremenskoj skali od nekoliko
  sekundi (1/f, ne beli).
- **Reset jitter** komparatora: svaki ciklus saw-core-a ima različit
  "time-of-flight" tokom reset-a (par nanosekundi). To stvara **inherentni
  spread u visokim harmonicima** — spektar je uvek malo različit od prethodnog
  ciklusa.

Digital nema ništa od ovoga besplatno. Savršen DCO daje phase-coherent
cancellation kad se dva glasa sretnu u unisono i zvuči *mrtvo*, ne *čisto*.

**Pravilo**: živost dolazi iz nedeterminizma ograničenog na muzikalne ose
(pitch, phase, amplituda) u pink-noise band-widthu. Beli šum = kvar; pink/flicker
šum = *nešto živi unutra*.

### 1.2 Beating — aritmetika unisonoa

Dva saw-a na `f1` i `f2`, `Δ = |f1−f2|`:

- u vremenu: amplituda se modulira sa `Δ` Hz (AM envelope)
- u spektru: svaka harmonika `n·f` postaje par `n·f1, n·f2` razdvojenih `n·Δ`
- na 10. harmoniki, 5-cent detune (`Δ ≈ 0.3%`) postaje ~3% razdvojenosti →
  **perceptivni chorus koji nije efekat, to je oscilator**

Energija se razliva po susednim frekvencijama umesto da bude tačkasta. Uvo
interpretira razlivene linije kao više izvora → "dubina" koju jedan perfektan
oscilator ne može dati ni sa najboljim reverb-om.

### 1.3 Filter — srce topline

Filter topologije se razlikuju jer im se **nelinearnost nalazi na različitim
mestima u feedback petlji**:

| Topologija | Gde saturira | Karakter |
|---|---|---|
| Moog ladder (4 BJT) | svaka stepenica gm, sat. u povratku | zgusnut low-mid, glatka rez. |
| OTA (SSM2040, CEM3320, 2044) | gm pada sa I_abc | vokalni, otvorenija rez. |
| Diode ladder (303, EMS) | pn-junction soft clip u ladder-i | agresivan squelch, breathing |
| Sallen-Key (CS80, ARP) | op-amp asim. clip | mekši vrh, manji bite |

Rezonanca nije samo Q, to je nelinearni oscilator uronjen u signal. Harmonički
sadržaj signala **intermodulira sa sopstvenim filtrom** → sidebands koje nisu
ni u oscilatoru ni u signalu. Ovo je fizika `Pec`-a (pre-filter load → filter
strain).

### 1.4 Nelinearnost — harmonijska poligrafija

- **Simetričan waveshaper** (tanh, atan): samo neparne harmonike (H3, H5, H7)
- **Asimetričan** (diode rectifier, class-A sa DC offsetom, transformer B-H):
  i parne (H2, H4)

Psihoakustički: H2 = oktava fundamenta → *muzikalno*. H3 = kvinta+oktava →
muzikalno. H5, H7 postaju disonantne.

- Transformator + class-A izlaz → "topao" (H2 dominantno)
- Hard-clipping op-amp → "grub" (H3+ dominantno)

Pri ~5% THD: dominantno H2 = warm, dominantno H3 = rich, H5/H7 = harsh.

Mamut `final character stage` je tačno mesto gde se ovo bira.

### 1.5 Power rail — "glue" bez kojeg voices ne zvuče kao instrument

Analog voices dele `±12 V` rail preko konačnog PSRR (~60–80 dB open loop,
znatno lošije kod diskretnih stepena). Kad osam glasova udari note-on:

- envelope attack izvlači par mA
- rail mikro-propadne za ~0.5–5 mV
- svi drugi aktivni glasovi dobijaju tu modulaciju na bias tačkama
- voices se međusobno čuju kroz power supply ~−40 do −60 dB

**Nije bug**. Mehanizam zbog kojeg akord zvuči kao jedno telo, a ne 8 izolovanih
sinusoida. Analogno sa čelom u kvartetu — vazduh i pod ih mehanički spajaju.
Digital: voices su bit-perfectno izolovane osim ako se eksplicitno ne modelira
`bus saturation` u voice sum stage-u. DSP spec to traži: "voice summing must
not be treated as a neutral add-only mixer".

### 1.6 Komponentni šum — tekstura koja popunjava dead space

- Johnson termalni šum: `4kTR·Δf` → bel, kroz RC mreže postaje **pink**
  (~−3 dB/oct)
- 1/f flicker šum: dominira ispod ~1 kHz
- Ekspo par šum modulira *pitch*, ne amplitudu → zvuči kao živost, ne šum

Tipičan nivo ~−75 do −85 dBFS. Uvo razlikuje "praznu tišinu" (digital) od
"vazduha" (analog) već ispod praga svesnog slušanja.

### 1.7 Kondenzatori i dielektrička apsorpcija

Elektrolitski i keramički caps-i imaju **memory effect**: naboj deponovan u
dielektriku polako difunduje nazad. Na saw-core-u (30–30000 Hz) ovo dodaje
fini harmonic distortion zavisan od prethodne istorije signala —
path-dependent distorcija. TB-303 značajan deo karaktera duguje jeftinim
keramičkim caps-ovima u filter core-u.

## 2. Psihoakustika — zašto "toplo" i "masno" nisu subjektivni

### 2.1 Fletcher-Munson / ISO 226

Uvo je najosetljivije na 2–5 kHz, mnogo manje na <100 Hz i >10 kHz.
Posledica: harmonic enhancement u 2–5 kHz opsegu povećava perceptivnu
glasnoću bez dizanja RMS-a/peak-a. Zato soft-saturated signal *zvuči glasnije*
od istog signala na istom peak-u.

`Pec` snaga = više energije u band-u na koji je uvo osetljivo, low-end ostaje
neklipovan.

### 2.2 Missing fundamental

Harmonici na `2f, 3f, 4f, 5f` sa slabim ili odsutnim `f` → uvo rekonstruiše
`f`. Sub ne mora biti na punom nivou — dovoljno je imati dominantan spektar u
80/120/160/200 Hz i bas se čuje niže nego što jeste.

Rešava fizički problem malih monitora. Mamut sub mora reinforcirati low-mid
oktavu (80–200 Hz) gde masa zaista ulazi u akord.

### 2.3 Kritični bandovi i beating

Kritični band uva (~1/3 oktave u mid-range-u): dve tonove razdvojene <~15%
centra banda se integrišu kao **jedan modulisan ton**. Unison detune 3–15
centi upada u ovu zonu: uvo zna da nešto nije ravno, ali ne razdvaja izvore
→ *mnogo jednog glasa* = širina bez rascepa.

- >20 centi: dva odvojena glasa (off-tune)
- <1 centa: nestaje
- **Sweet spot: 3–8 centi spread, slow random walk ~0.1–0.5 centi/s** — tu
  živi `Horizont`.

### 2.4 Spectral centroid i tilt — warmth kao merljiva veličina

- **Warmth** = centroid ispod ~2 kHz sa prirodnim −3 do −6 dB/oct roll-off-om
  ka visokom kraju, plus dominantne parne harmonike
- **Brightness** = centroid iznad 3 kHz
- **Harshness** = dominantne H5, H7, H9 bez prigušenja

Analogni output se prirodno low-pass filtrira kroz kapacitivnost kola, žica,
transformatora → implicitni −6 dB/oct tilt je *ugrađen*. Digital mora
eksplicitno da ga dodaje.

### 2.5 Fazna koherencija i "life"

JND za fazu između dva harmonijski bogata izvora: ~3–5° u mid-range-u. Dva
savršeno fazno-lock-ovana saw-talasa → peak u amplitudi na svakom reset-u
(konstruktivno) + cancellation u suprotnoj fazi = prepoznatljivo *mlitavo/mrtvo*.

Analogni drift osigurava da phase relationship stalno lagano šeta → uvo
registruje kao "diše".

Digital: ako ne randomizuješ phase na note-on i ne dodaš tiho random walk na
pitch, unison = destructive interference = jedan tanak oscilator.

### 2.6 Maskiranje — zašto analog podnosi distorziju bolje

Simultaneous masking: jak ton prikriva slabije u susednim kritičnim bandovima.
Analogna distorzija je usidrena u sopstveni spektar signala (dolazi *iz*
signala) → samo-maskirajuća. Digitalni aliasing generiše frekvencije koje
nisu harmonički povezane sa signalom (foldback) → neusidrene → nisu maskirane
→ zvuče kao grube nečistoće čak i pri niskom nivou.

Razlog zašto naivni digital oscilator zvuči "cheap" a analog zvuči
"dirty-but-right" pri istom THD-u.

## 3. Digital-analog razlika — operacionalno

| Aspekt | Analog | Naivni digital | Dobar VA |
|---|---|---|---|
| Pitch preciznost | ±2–10 centi drift | <0.001 centi | random-walk ±1–5 centi |
| Phase na note-on | random (cap state) | fiksan (sample 0) | randomizovan [0, 2π) |
| Spektar saw-a | band-limited analog LPF | aliased ili BLEP | poly-BLEP/BLIT |
| Filter nonlin. | inherentna | linearni IIR | ZDF + tanh saturation |
| Noise floor | pink, −75 dBFS | −∞ (float) ili dither | injected pink/1f |
| Voice crosstalk | PSRR, ground, rail | nula | bus saturation model |
| Transient | ~10–100 µs rise | bit-precizno | oversampling oko nelin. |
| Component jitter | inherentno | nula | per-voice offset |
| Temperature | merljiva (tempco) | nepostojeća | slow sinusoid + noise |
| Dielectric | path-dependent | nula | biquad sa hysteresis-om |
| Transformer | B-H loop nelinearan | ne | dinamički frek-zavisni saturator |

**Suština digitalnog problema**: svaka "imperfekcija" mora biti eksplicitno
modelirana. Ako nijedna nije, digital zvuči kao crtež analoga — tačan na
mreži, bez života.

## 4. Implementacija za mamut-sint-sw

### 4.1 Oscilatori (EPM1)

- **Band-limited waveforms**: PolyBLEP minimum; BLIT/BLAMP za pulse-edge
  tačnost. `osc2` kao rupture carrier: hard sync sa BLEP-korekcijom na reset
  event-u. Sync = dve diskontinuiteta u istom frame-u → bez korekcije audible
  aliasing na high pitch-u.
- **Per-voice drift model**: 2-pole LPF-ovan white noise, σ ≈ 2–5 centi,
  cutoff ≈ 0.3 Hz. Po glasu zaseban state.
- **Temperature model**: spor sinusoidal walk (~60–120s period) na svim
  glasovima sa zajedničkim fazom → ceo chassis "diše" zajedno, ne svaki glas
  nasumično.
- **Sub osc**: ne čist sinus oktavu niže. Square sa blagim LPF-om daje
  H1+H3+H5 → u kombinaciji sa `osc1`/`osc2` popunjava low-mid masu `Pec`-a,
  virtuelni fundamental rekonstruiše bas.
- **Unison detune**: 3–8 centi, phase randomized.

### 4.2 Mixer / pre-filter load (`Pec` primary)

Spec ispravno identifikuje kao "pressure site". Implementacija:

```
pre_filter_load = soft_saturate(sum(osc_i * mix_i), drive_from_pec_macro)
where soft_saturate(x, d) = tanh(d*x + ε_asym(d)) / tanh(d)
```

Asimetrija `ε_asym` je ključ H2 naglaska → topao karakter. Bias term
0.05–0.15 pri visokom drive-u daje parnu dominaciju.

### 4.3 Filter (sve tri uloge)

Jedan filter koji pokriva `Horizont` → `Pec` → `Baklja`:

- **ZDF trapezoidal integrator** (Vadim Zavalishin) — tačna analog emulacija
  bez bilinear pre-warp grešaka.
- **Saturation unutar feedback petlje**, ne samo na ulazu/izlazu. Fizika Moog
  ladder-a: svaka stepenica u petlji je sopstveni soft-limiter.
- Nizak rez = linearan, otvoren (`Horizont`)
- Mid rez + drive = kompresovan, loaded (`Pec`)
- Near self-oscillation + drive = strain, signal puca (`Baklja`)

Praktično: ZDF 4-pole ladder sa tanh u svakoj stepenici. Cost ~3–4x linear
IIR, ali to je razlog postojanja software-a u EPM1 eri.

### 4.4 Voice summing — "glue"

Spec: "must not be treated as a neutral add-only mixer". Dva načina:

1. **Bus saturation**: soft compressor + tanh na sumi svih glasova pre final
   stage-a. Čak i kad pojedinačni glasovi ne kliphuju, suma ulazi u zonu gde
   se ~2% parnih harmonika injektuje — osam glasova u akordu postaje jedno
   telo.
2. **Cross-voice modulation** (ambiciozno): blaga AM modulacija (~−60 dB)
   proporcionalna ukupnoj sumi. Psihoakustika identična PSRR crosstalk-u u
   analognom.

### 4.5 Final character stage (mandatory)

Srce onoga što je "masnoća" i "snaga":

- **Dynamic frequency-dependent saturation**: više sat. u low-mid
  (50–500 Hz), manje u visokom (>5 kHz). Dva paralelna saturatora sa
  komplementarnim EQ, zatim sum.
- **Asimetrija** (H2 pump): bias injection zavisan od apsolutne vrednosti
  signala.
- **Transformer model**: B-H hysteresis — integrator sa memory-termom koji
  generiše path-dependent distorziju. Diode model (Chua-style) ili
  jednostavnije, soft_saturate sa feed-forward smoothed envelope-om koji
  modulira drive.
- **Headroom shaping**: peak limiter sa *veoma spore* attack (~10 ms),
  release ~100 ms. Ne kompresor u mastering smislu — suptilan "loaded"
  osećaj kao da je output transformator blizu zasićenja.

`Gravitacija` fizički ulazi u zvuk: macro kontrola skalira `drive`,
`asymmetry`, `headroom_threshold` zajedno. Low = linearan prolaz, high =
transformer na kolenu.

### 4.6 Anti-aliasing i oversampling

Svaki nelinearan stepen **mora** oversampling 2–8x, inače foldback aliases
= "digital dirt". Poredak:

1. Upsample (polyphase FIR, 4x ili 8x)
2. Nelinearnost (saturation, asimetrija, sync reset)
3. Decimation FIR (anti-alias LPF pre downsample-a)

Minimum na: osc sync path, filter feedback saturaciji, final character stage.
Najveći compute budget u ceo DSP-u.

### 4.7 Psihoakustički "hidden moves"

- **Pink noise injection** ~−75 do −85 dBFS u signal path (ne u safety stage).
  "Vazduh" ispod praga svesnog slušanja.
- **Phase randomization na note-on**: uniform [0, 2π) po oscilatoru. DSP
  spec traži determinizam po sample rate i block ordering-u → phase random
  seed-uj iz deterministic PRNG-a (note number + voice index) → reproducibilan
  ali zvučno živ.
- **Envelope slope variation**: po glasu ±2–5% na attack/release time-u.
  Različite RC konstante između glasova u pravom analognom.
- **Frequency-dependent group delay** u final stage (nije flat fazno): mala
  all-pass sekcija na 2–5 kHz → smooth transient koji nije bit-perfektno
  oštar.

## 5. Mapiranje na Mamut vokabular

| Projekat | Fizika | Implementacija | Psihoakustika |
|---|---|---|---|
| `Horizont` | wide stabilno detune, headroom, low-mid disciplina | 3–8 cent spread, LPF low Q, bez saturacije | beating u critical band = wide image |
| `Pec` | pre-filter load + filter drive + H2-dominantna saturacija | tanh+bias u mixer-u, ZDF filter mid drive, transformer-like final | virtual fundamental + 2–5 kHz energy = snaga |
| `Baklja` | sync tearing + crossmod sidebands + asim. saturacija u filter self-osc zoni | hard sync sa BLEP, cross-FM, ZDF filter high Q sa tanh u feedback-u, asym. waveshaper | odd-H dominacija + inharmonic sidebands = tearing, ivica |
| `Gravitacija` | macro koji koreliše drive, asymmetry, headroom, cross-mod depth | jedan scalar mapiran na 5–8 DSP param. kroz curve lookup | open → loaded → strained = kontinualna mapa Horizont → Baklja |

`analog-learning-roadmap`: "listening as measurement". Pravi posao je da se
svaka od ovih tabela verifikuje A/B slušanjem protiv stvarnog
Moog/Prophet/Oberheim referentnog hardvera. Fizika daje šta *treba* raditi;
uši određuju *koliko*.

## 6. Skeleton teorije

- **Masnoća** = harmonička gustina (raspakovana detune-om + bogatom
  saturacijom)
- **Toplina** = parna-harmonička dominacija + spektralni tilt
- **Snaga** = energija u 2–5 kHz uz kontrolisan headroom
- **Živost** = sub-threshold randomness na muzikalnim osama
  (pitch/phase/amplitude)

Analog dobija sve četiri besplatno kroz nesavršenost. Digital mora svaku da
zasluži. `mamut-sint-sw` arhitektura pokazuje da projekat to zna: mandatory
final stage, pre-filter load kao pressure site, sync-first rupture umesto
chaotične distorzije.

## 7. Konkretni brojevi za V1 target

| Parametar | Vrednost | Razlog |
|---|---|---|
| Unison detune spread | 3–8 centi | critical band sweet spot |
| Pitch random walk | σ=2 centi, τ≈3s | 1/f drift model |
| Phase on note-on | uniform [0, 2π), seeded | živost bez lom determinizma |
| Pink noise floor | −75 do −85 dBFS | vazduh ispod praga svesti |
| H2 bias pri nominal | 1.5–2.5% THD | warm bez grubosti |
| H2 bias pri high drive | 8–15% THD | Pec loaded |
| Filter oversampling | 4x minimum, 8x za self-osc | anti-foldback |
| Voice crosstalk sim | −50 ± 5 dB | PSRR ekvivalent |
| Envelope slope variance | ±3% po glasu | RC tolerance |
| Temperature drift period | 60–120 s | thermal equilibration |
| Final stage attack | ~10 ms | transformer feel |
| Final stage release | ~100 ms | loaded sustain |

Ove brojeve smatrati startnim tačkama za A/B slušanje, ne zakonima.
