# GFM Related Concepts Research

Date: 2026-05-09

Status: research note, not an implementation contract.

## Local Reading

In this repo, GFM is best treated as an opt-in field/lattice layer:

- a small deterministic lattice (`GfmLattice16` / `GfmLattice<16,16>`);
- patch-identity driven program selection;
- performance programs such as Horizont, Pec, Baklja, Gravity-style behaviors;
- state and diagnostics around strain, rupture, health, quarantine/recovery;
- a bounded layer mixed into the primary synth path, rather than the whole synth engine.

The useful framing is not "another physical modelling synth". The stronger framing is:

> GFM is a performable nonlinear field layer: a bounded, identity-aware lattice
> that adds controllable stress, rupture, recovery, and emergent motion to the
> core synth voice.

## Closest Concept Families

### 1. Digital Waveguide Mesh / FDTD Physical Modelling

This is the closest technical family for "cells with local propagation".
Digital waveguide meshes and finite-difference time-domain models simulate
energy moving through strings, plates, membranes, rooms, vocal tracts, and
other physical systems.

How it maps to GFM:

- Shared idea: local state, propagation, wave/energy movement, bounded update.
- Difference: GFM is not trying to be a faithful plate/string/room simulator.
  It is a musical field layer with program identity and safety state.
- Borrowable ideas: energy accounting, damping laws, boundary conditions,
  pickup/excitation placement, stability criteria.

References:

- Damian Murphy, Antti Kelloniemi, Jack Mullen, Simon Shelley, "Acoustic modeling using the digital waveguide mesh", IEEE Signal Processing Magazine, 2007. https://pure.york.ac.uk/portal/en/publications/acoustic-modeling-using-the-digital-waveguide-mesh
- Stefan Bilbao et al., "Physical modeling, algorithms and sound synthesis: The NESS Project", Computer Music Journal, 2020. https://www.research.ed.ac.uk/en/publications/physical-modeling-algorithms-and-sound-synthesis-the-ness-project
- Madrona Labs Kaivo product notes, useful commercial reference for FDTD physical modelling as an instrument. https://www.madronalabs.com/products/kaivo

### 2. Nonlinear Modal / Mode-Coupling Synthesis

This family is close to the idea that the instrument state can move energy
between modes, instead of keeping all resonators independent.

How it maps to GFM:

- Shared idea: coupled parts of a resonant system can exchange energy and
  produce nonlinear, perceptually meaningful side effects.
- Difference: modal synthesis usually operates as a resonator/filter-bank
  model; GFM is a lattice/program layer that can modulate or color the main
  voice.
- Borrowable ideas: coupling matrices, named energy-transfer paths, nonlinear
  stress/impact behavior, stable controllability.

References:

- Samuel Poirot, Stefan Bilbao, Richard Kronland-Martinet, "A simplified and controllable model of mode coupling for addressing nonlinear phenomena in sound synthesis processes", EURASIP Journal on Audio, Speech, and Music Processing, 2024. https://link.springer.com/article/10.1186/s13636-024-00358-2
- Michele Ducceschi, Stefan Bilbao, Craig J. Webb, "Real-time modal synthesis of nonlinearly interconnected networks", DAFx 2023 / University of Edinburgh record. https://www.research.ed.ac.uk/en/publications/real-time-modal-synthesis-of-nonlinearly-interconnected-networks

### 3. Dynamical Systems / Nonlinear Feedback Synthesis

This family covers small nonlinear systems, delay-free loops, virtual analog
models, and physical models whose interesting behavior comes from feedback and
nonlinearity.

How it maps to GFM:

- Shared idea: nonlinear state can be musically useful if it is computable,
  bounded, and real-time safe.
- Difference: much of the literature targets circuits or generic ODE systems;
  GFM targets playable identity/gesture behavior in this instrument.
- Borrowable ideas: explicit update forms, no hidden iterative solvers in the
  audio path, bounded nonlinear response, real-time suitability criteria.

References:

- David Medine, "Dynamical Systems for Audio Synthesis: Embracing Nonlinearities and Delay-Free Loops", Applied Sciences, 2016. https://www.mdpi.com/2076-3417/6/5/134

### 4. Wave Terrain / Agent Terrain Synthesis

Wave terrain synthesis generates sound or control motion by traversing a
surface or terrain. Later work connects terrains with agents and live coding.

How it maps to GFM:

- Shared idea: a 2D field/terrain plus trajectory can become timbre, gesture,
  or modulation.
- Difference: in wave terrain synthesis the terrain is often a read surface;
  in GFM the field has its own evolving state and health.
- Borrowable ideas: visual engine UI, trajectory controls, terrain inspection,
  named paths through the field.

References:

- Stuart James, "Possibilities for Dynamical Wave Terrain Synthesis", Australasian Computer Music Conference, 2003. https://www.stuartgjames.com/uploads/1/7/4/5/17453311/2003jamesacmc.pdf
- Gerard Roma, "Agent-Based Music Live Coding: Sonic adventures in 2D", Organised Sound, 2023. https://www.cambridge.org/core/journals/organised-sound/article/agentbased-music-live-coding-sonic-adventures-in-2d/C97704355CB21A992CBC32A8E073116D

### 5. Cellular Automata / Reaction-Diffusion / Coupled Lattices

This is closest to the "emergent pattern" side: local rules, discrete space,
propagation, stability/instability, and pattern recovery.

How it maps to GFM:

- Shared idea: local cell updates can generate global behavior, and performer
  controls can steer the system without directly specifying every event.
- Difference: much CA music is generative composition or parameter control;
  GFM is intended as a live audio-engine color layer.
- Borrowable ideas: update rule families, rupture/recovery visual language,
  bounded chaos, pattern metrics, lattice diagnostics.

References:

- Eduardo R. Miranda, "Cellular Automata Music: From Sound Synthesis to Musical Forms", in Evolutionary Computer Music, 2007. https://researchportal.plymouth.ac.uk/en/publications/cellular-automata-music-from-sound-synthesis-to-musical-forms
- Eduardo R. Miranda, "Cellular Automata Synthesis of Acoustic Particles", ICMC Proceedings, 1995. https://researchportal.plymouth.ac.uk/en/publications/cellular-automata-synthesis-of-acoustic-particles
- Dave Burraston, Ernest Edmonds, "Cellular automata in generative electronic music and sonic art: a historical and technical review", Digital Creativity, 2005. https://www.tandfonline.com/doi/abs/10.1080/14626260500370882
- Andrew Martin, "Reaction-diffusion systems for algorithmic composition", Organised Sound, 1996. https://www.cambridge.org/core/journals/organised-sound/article/reactiondiffusion-systems-for-algorithmic-composition/A57A2F7A6C459983C474E7E4894971E1

### 6. Exciter / Resonator Instruments

Commercial and open instrument references are useful for UX and musical
framing, even when their synthesis core is different.

How it maps to GFM:

- Shared idea: user-facing controls should describe material, excitation,
  damping, structure, brightness, instability, and response.
- Difference: Rings and related instruments are mostly resonator-centered;
  GFM is a layer on top of the Mamut voice and identity system.
- Borrowable ideas: do not expose raw math first; expose performable material
  concepts and put diagnostics behind an engine/developer view.

References:

- Mutable Instruments Rings manual. https://pichenettes.github.io/mutable-instruments-documentation/modules/rings/manual/
- Mutable Instruments Rings original blurb. https://pichenettes.github.io/mutable-instruments-documentation/modules/rings/original_blurb/
- Softube Rings product page, useful concise commercial wording around exciter/resonator framing. https://www.softube.com/us/plug-ins/mutable-instruments-rings
- Madrona Labs Kaivo. https://www.madronalabs.com/products/kaivo

## False Friend: Wave Field Synthesis

Wave Field Synthesis sounds close by name, but it is mostly a spatial audio
reproduction technique: many independently controlled loudspeakers recreate
wavefronts in physical space. It is not a timbral field/lattice synthesis
core in the GFM sense.

It is still useful as a naming caution. "Field" in GFM should be explained as
an internal playable modulation/color field, not as room-scale wavefront
reproduction.

References:

- EMPAC Wave Field Synthesis Array. https://empac.rpi.edu/program/research/wave-field-synthesis
- Analytic Methods of Sound Field Synthesis, Jens Ahrens supplementary materials. https://www.soundfieldsynthesis.org/

## Working Conclusion

GFM is most defensible as a hybrid of:

- local lattice / wave-propagation thinking;
- nonlinear modal energy transfer;
- bounded dynamical systems;
- terrain/agent control metaphors;
- CA/reaction-diffusion style emergent patterning;
- performance-oriented exciter/resonator UX.

The innovation is not that every ingredient is new. The useful novelty is the
instrument-level combination:

- patch identity chooses field behavior;
- field state has health, rupture, strain, and recovery;
- the layer is opt-in and bounded;
- the result is playable as character, not exposed as a physics simulator.

## Next Design Questions

- Should GFM expose a visible field/terrain view in the Engine tab?
- Should each program have a named energy-transfer profile instead of ad-hoc
  constants?
- Should rupture/recovery become a first-class performance control, or remain
  mostly automatic?
- Can we define a small coupling matrix API for GFM without increasing hot-path
  allocation or code complexity?
- Should GFM output be audio-rate, control-rate, or a mixed model with strict
  boundaries?

