# Mamut: Masnoca, Punoca, Snaga i Toplina

## Svrha

Ovaj dokument sabira jednu ozbiljnu radnu analizu pitanja:

- sta analognim sintisajzerima daje punocu
- sta daje masnocu
- sta daje snagu
- sta daje toplinu
- kako se to odnosi na `Mamut EPM`
- sta je vec dobro pogodjeno u `mamut-sint-sw`
- gde je sledeci veliki dobitak

Dokument je pisan iz ugla trenutnog `EPM1` softverskog engine-a u
`mamut-sint-sw`, ali namerno ga cita u paru sa `EPM2` hardverskom linijom u
`/home/dev/sel4/mamut-sint-hw`.

## Kratak Zakljucak

Analogni sint ne zvuci punije zato sto je "analogni" kao magijska etiketa.
Zvuci punije kada vise malih fizickih nelinearnosti i ogranicenja rade zajedno:

- izvorni talas nije savrseno sterilan
- odnos oscilatora nije potpuno mrtav ni potpuno haotican
- mikser i filter ulaz nose opterecenje i blago zasicenje
- filter ne menja samo cutoff, nego i raspodelu energije i ponasanje pri pritisku
- izlazni stepen menja headroom, asimetriju i gustinu low-mid opsega
- glasovi nisu kopije bez identiteta, ali nisu ni raspad sistema

Drugim recima:

- punoca = raspodeljena gustina kroz ceo lanac
- masnoca = masa u fundamentalu i low-mid zoni plus blago grupisani parcijali
- toplina = redistribucija harmonika i smanjenje staklastih ivica
- snaga = veca perceptivna blizina, manji crest factor, veca koncentracija energije

To je sustina.

## Mamut Kontekst

`Mamut EPM` vec ima zdravu osnovnu podelu:

- `EPM1` u `mamut-sint-sw` je trenutna softverska realizacija
- `EPM2` u `mamut-sint-hw` je analogna hardverska linija

Relevantan stav hardverske linije je jasan:

- analogni voice path
- digitalni control brain
- digitalni deo za recall, allocaciju, kalibraciju i makroe
- analogni deo za weight, nonlinearity i voice character

To je dobra podela odgovornosti. Ne gura digitalni sloj da "glumi analog" gde
ne treba, ali mu daje tacno ono u cemu je superioran:

- stabilnost
- recall
- koordinaciju
- tuning
- kalibraciju
- performans makroe

## Fizicki Izvori Punog Analognog Zvuka

### 1. Oscilator nije samo talas, nego mehanizam

Kod analognog VCO-a pitch i timbar nastaju iz fizike:

- expo konverter pravi struju iz kontrolnog napona
- ta struja puni timing kondenzator
- prag odlucuje kada se reset desava
- reset stepen prazni rampu
- sync umece silu u taj resetni odnos

To znaci da izvor zvuka nije matematici idealan generator. On je mali dinamicni
sistem sa:

- granicama
- kasnjenjima
- termickom zavisnoscu
- zavisnoscu od tolerancija
- zavisnoscu od reset brzine i reset dubine

U `mamut-sint-hw` je bas to prepoznato kao prvi ozbiljan izvor karaktera.
P1 lanac je eksplicitno:

- `CV_IN -> TUNE_SUM -> EXPO_OUT -> RAMP_NODE -> THRESHOLD_NODE -> RESET_CTRL -> SAW/PULSE`

vidi:

- `sim/ngspice/p1-vco/60-integrated-vco-chain.cir`
- `docs/p1-vco-simulation-notes.md`

Najvaznija lekcija tog rada je da reset prozor i reset stepen nisu sitan detalj
nego muzicki parametar. Ako se donja tacka rampe i brzina resetovanja promene,
menja se:

- period
- harmonijski sadrzaj
- osecaj "napetosti"
- osecaj "griza"
- reakcija na sync

To je veoma blizu onome sto ljudi slusno opisuju kao zivot, toplinu ili snagu.

### 2. Harmonici nastaju i kroz zasicenje i kroz opterecenje

Analogni mikser, filter ulaz, VCA i izlazni stepen ne ponasaju se kao savrsene
linearne matrice. Kada ih guras:

- menjaju odnos harmonika
- blago kompresuju vrhove
- dodaju asimetriju
- premestaju energiju ka nizim i srednjim harmonijskim komponentama

Ako je to uradjeno umereno, rezultat nije "distortion" u gitaristickom smislu,
nego:

- veca gustina
- manja praznina u low-mid zoni
- manji utisak staklenog vrha
- osecaj da je zvuk blizi i tezi

### 3. Termika, tolerancije i mismatch

Kod analognog sinta deo "zivota" dolazi iz toga sto sistem nije savrseno
ponovljiv.

Ali ovde postoji bitna razlika:

- dobra analogna zivost nije raspad
- dobra analogna zivost je kontrolisana divergencija

Premalo divergencije:

- sterilan zvuk
- previse tvrd i "pljosnat" identitet

Previse divergencije:

- mutna intonacija
- raspad akorda
- gubitak centra

Zato je pravi cilj:

- pitch spine ostaje citljiv
- parcijali i odnosi ostaju blago pokretni
- voice individuality postoji, ali ne unistava muzicku strukturu

## Psihoakustika: Zasto To Cujemo Kao "Masno" i "Toplo"

### 1. Low-mid organizacija

Ljudi "masnocu" retko cuju kao obican boost bassa.
Mnogo cesce je cuju kao:

- jak fundamental
- dovoljno drugog i treceg harmonika
- prisutnost u low-mid opsegu
- odsustvo supljine izmedju basa i viseg srednjeg opsega

Zato mnogi tanki sintovi nisu stvarno "bez basa", nego bez pravog centra
gravitacije u low-mid regionu.

### 2. Blaga nelinearna kompresija

Signal koji ima blago zasicenje i blago skracen crest factor cesto zvuci:

- jace
- blize
- vece
- "skuplje"

i kada peak metriki ne deluje drasticno drugacije.

To je vazno: snaga nije isto sto i glasnoca.
Snaga je cesto perceptivna koncentracija energije.

### 3. Pokret bez haosa

Dva skoro ista oscilatora, ili jedan oscilator i sub, stvaraju beating,
zblizavanje i razmicanje parcijala.
Ako je to blago i organizovano, uho to cita kao:

- sirinu
- telo
- zivot

Ako je to preterano, uho to cita kao:

- nesigurnost
- mutljavost
- los tracking

Tu lezi jedna od najvecih razlika izmedju dobrog analognog instrumenta i lose
digitalne "analog simulacije": nije dovoljno dodati random drift. Potrebna je
kontrolisana, spora, muzicki korisna divergencija.

### 4. Toplina nije isto sto i tamno

Mnogi ljudi opisuju topao zvuk kao "mracniji", ali to je samo deo price.
Prava toplina obicno znaci:

- manje ostrih i neprijatnih ivica
- bogatije nize harmonike
- manja krhkost vrha
- vise "mesa" oko centra tona

Zvuk moze biti topao i dalje otvoren.
Zvuk moze biti taman i ipak mrtav.

## Kako Se Ovo Vec Vidi U Mamut SW

`mamut-sint-sw` vec ne razmislja o tonu kao o jednom filteru i jednom drive-u.
To je velika prednost.

### 1. Dobar identitetski model

`mamut-identity` razlaze makroe u skrivena stanja:

- `mass`
- `strain`
- `headroom`
- `body_focus`
- `rupture_threshold`
- `rupture_response`
- `spatial_dispersion`

To je odlican model jer:

- punoca nije jedan parametar
- toplina nije jedan parametar
- opasnost nije jedan parametar

`Gravitacija` zato moze da bude pravi organizam-signal, a ne obican cutoff-plus-drive
macro.

### 2. Dobar tonalni raspored po lancu

U engine-u su vec prisutne prave tacke pritiska:

- `sub` se pojacava preko `mass`
- pre-filter miks ide kroz saturaciju
- filter dobija drive i strain
- postoji per-voice soft clipping
- postoji poseban globalni final body stage
- low-mid je eksplicitno tretiran kao tonski centar, ne kao EQ flaster

To je zdrava arhitektura, jer "Pec" i "Gravitacija" ne zive samo na kraju lanca.

### 3. Patch bank vec autoruje telo iz izvora

Patch-evi kao `Furnace Choir` i `Gravity Wake` vec pokazuju da se masnoca ne
trazi samo u FX-u nego u:

- nivou suba
- body mix-u
- pre-filter drive-u
- final stage body drive-u
- low-mid emphasis-u
- sync/crossmod odnosu

To je dobar znak da je tonalna filozofija vec usla u konkretne odluke.

## Gde Je SW Jos Uvek "Previse Digitalan"

Ovo je najvazniji tehnicki caveat.

Danasnji `mamut-sint-sw` dobija mnogo "analognog osecanja" iz:

- dobrog rasporeda mase
- saturacije
- filter drive-a
- final body stage-a

ali manje iz samih oscilatora.

Trenutni oscilatori su i dalje pre svega:

- fazni akumulator
- idealni saw/pulse/triangle oblici
- vrlo direktan `hard_sync`

To znaci:

- oscillator fizika jos nije glavni izvor karaktera
- glavni izvor karaktera je trenutno vise downstream
- analogna punoća se trenutno "komponuje" posle izvora vise nego u samom izvoru

To nije problem za ovu fazu proizvoda.
Ali jeste glavno mesto gde ce sledeci skok u kvalitetu zvuka najverovatnije nastati.

## Analogne i Digitalne Razlike, Bez Mistifikacije

### Analogno je jace kada:

- zelis distribuiranu nelinearnost kroz vise blokova
- zelis da izvor zvuka sam nosi karakter
- zelis da reset, threshold, current i bias budu deo tona
- zelis da dinamika opterecenja i headroom-a zivi u samom glasu

### Digitalno je jace kada:

- zelis recall
- zelis stabilnu polifoniju
- zelis preciznu modulaciju
- zelis patch autoring i macro koordinaciju
- zelis repeatable startup i servisnu disciplinu

### Najbolji rezultat za Mamut

Za `Mamut EPM`, najzdravija putanja je upravo hibrid:

- analogni core za weight, nonlinearity i voice character
- digitalni nervous system za control, tuning, memory i macro orchestration

To je vec eksplicitna namera `EPM2`, i dobra je.

## Kako "Masnoca" Treba Da Se Razume Unutar Mamut-a

Ako hocemo radni jezik, predlazem sledecu internu podelu:

### Punoca

Kad akord ili ton nema osecaj rupe.
Dolazi iz:

- dovoljno gustog izvora
- zdravog low-mid centra
- dobrog pre-filter sabiranja
- final body concentration

### Masnoca

Kad ton ima "meso" i tezinu, ne samo sirinu.
Dolazi iz:

- sub prisustva
- grupisanja parcijala
- low-mid emphasis
- blage kompresije i saturacije

### Toplina

Kad ton nema krhke, staklaste, suve ivice.
Dolazi iz:

- mekog clipping-a
- asimetrije
- jacih nizih harmonika
- manjeg perceptivnog "ice pick" vrha

### Snaga

Kad ton deluje blize, fizicki prisutnije i autoritativnije.
Dolazi iz:

- manjeg crest factor-a
- vece koncentracije energije
- fokusiranijeg centra
- manje praznog headroom-a

## Konkretna Procena Trenutnog Mamut SW Lanca

Trenutni `EPM1` engine je jak u sledecem:

- tonalna semantika je dobra
- makroi su dobro raspakovani u fizicki razumljive posledice
- `Pec` i `Gravitacija` nisu svedeni na jedan distortion bus
- final stage je pravilno tretiran kao deo instrumenta, ne mastering dodatak
- `sub + mixer + filter + final stage` vec cine ozbiljan body sistem

Trenutni `EPM1` engine je jos slabiji u sledecem:

- oscillator source jos nije dovoljno "fizicki specifican"
- nema bogatu sporu korelisanu voice drift logiku
- sync je funkcionalan, ali jos nije mehanicki ubedljiv kao saw-core/reset fenomen
- analogni osecaj jos vise zivi u shaping-u nego u poreklu talasa

## Najvredniji Sledeci Potezi Za `mamut-sint-sw`

Ako je cilj veca punoca, masnoca i analogna ubedljivost bez razvaljivanja
postojece arhitekture, najbolji sledeci redosled bi bio:

### 1. Bolji oscillator model

Ne samo bolji talas, nego bolji mehanizam.

Pravac:

- saw-core inspirisan model
- finite reset behavior
- realniji sync reset
- blaga zavisnost talasa od frekvencije i pritiska

Ovo je najveca otvorena rupa izmedju danasnjeg SW zvuka i onoga sto `P1` HW rad
vec vrlo zdravo istrazuje.

### 2. Spora kontrolisana voice individuality logika

Ne random wow/flutter.
Nego:

- spori drift po glasu
- blago drugacije envelope konstante po glasu
- blago drugaciji filter bias po glasu
- korelisani, muzicki spori pokreti

Cilj:

- zvuk postaje zivlji
- akordi dobijaju telo
- bez raspada pitch spine-a

### 3. Jos bogatiji pre-filter mixer pressure

Pre-filter miks je vec dobar kandidat za glavni "Pec" generator.
Tu vredi dalje istraziti:

- level-dependent weighting
- asymmetry pre filter
- sub-specific drive law
- body-dependent saturation law

### 4. Dinamicki final stage

Trenutni final stage je dobar kostur.
Sledeci korak bi bio da se bias i headroom menjaju dinamicnije:

- nije isto kad jedna nota svira i kad pun akord pritisne bus
- nije isto kad je `Gravitacija` visoka ali `Bloom` takodje drzi prostor
- nije isto kad je `Baklja` armed i kad je active

Drugim recima:

- final stage treba jos vise da se ponasa kao organizam
- manje kao staticki waveshaper

## Zasto Je HW Linija Ovde Posebno Vazna Za SW

`mamut-sint-hw` nije samo "future hardware".
On je trenutno najbolji izvor istine za to gde nastaje pravi analogni karakter.

Najvrednije hardverske lekcije za SW su:

- expo law nije samo pitch law nego i karakter odnosa
- reset window i reset speed menjaju muzicki identitet
- sync je ubedljiviji kad je resetno/threshold prirode nego kad je cist API trik
- termika i mismatch treba da postoje kao kontrolisana muzicka sila, ne kao kvar

Zato bi bilo zdravo da `EPM1` vremenom ne "oponasa vintage nostalgiju", nego
da uzima konkretne mehanicke lekcije iz `P1` i prevodi ih u DSP.

## Radna Teza

Ako sve gore saberemo, najkraci radni princip za `Mamut` glasi:

> Masnoca nije FX.
> Masnoca je organizovana masa kroz ceo lanac.

I jos preciznije:

> Dobar analogni osecaj nastaje kada izvor, mikser, filter, VCA i final stage
> svi nose po malo realnog pritiska, umesto da jedan block na kraju glumi sve.

To je upravo smer u kome `Mamut EPM` vec deluje najzrelije.

## Prakticna Procena

`mamut-sint-sw` je trenutno blize ozbiljnom instrumentu nego generickom
softsynthu zato sto:

- ima dobru tonalnu filozofiju
- ima dobru raspodelu odgovornosti kroz lanac
- ne svodi identitet na preset copy ili post-FX lepak

Najveci sledeci dobitak nije:

- jos jedan FX
- jos jedan EQ
- jos jedan "warmth" knob

Najveci sledeci dobitak je:

- da oscillator i voice-level fizika nose vise karaktera pre nego sto signal
  stigne do filtera i final stage-a

To je mesto gde ce `EPM1` najvise profitirati od znanja koje `EPM2` vec sada
skuplja kroz `P1`.

