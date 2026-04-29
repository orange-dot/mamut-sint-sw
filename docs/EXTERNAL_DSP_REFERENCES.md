# External DSP References

This note tracks external DSP code that is useful for `EPM1` design work but is
not part of the `mamut-engine` runtime contract.

## AudioNoise

- Source: `https://github.com/orange-dot/AudioNoise`
- Owner: `orange-dot`
- Default branch: `main`
- Local checkout: `/home/dev/work-base-20260421/workspace/systems/AudioNoise`
- Local spike branch: `mamut-svf-stability-alsa-spikes`

`AudioNoise` is a guitar-pedal-oriented DSP reference. Its README describes a
single-sample-in, single-sample-out direction with basic IIR filters and delay
loops. For Mamut, this is most relevant as a small comparison point for
zero-latency pedal-style effects, post-voice tone shaping, and simple delay or
filter experiments.

Do not treat it as canonical Mamut DSP. Use it as a readable reference when
thinking about immediate sample-by-sample effects and simple control surfaces.
The current local branch documents three candidate transfer spikes: nonlinear
SVF/filter-drive, effect stability harness, and an ALSA live runner.

## dspc

- Source: `https://codeberg.org/catseyechandra/dspc.git`
- Lab archive: `/home/dev/work-base-20260421/archives/dspc-master.tar.gz`
- Lab snapshot: `/home/dev/work-base-20260421/forks/systems/dspc-master`
- Version in `CMakeLists.txt`: `1.2.0`
- Language/runtime: C library, optional JACK-facing helpers
- License: LGPLv3, per `README.md` and `LGPL-v3.0.txt`

Useful reference areas:

- IIR/filter design helpers: `dspc/iir_utils.h`, `dspc/butterworth_lr.h`,
  `dspc/iir_zb.h`
- PEQ block process shape: `dspc/path_peq.h`
- FIR design notes: `README-REMEZ.txt`, `dspc/remez.h`
- Block/vector processing patterns: `dspc/path_filtering_c.h`,
  `dspc/path_vec16_cintr.h`
- JACK client helper patterns: `dspc/jclient.h`, `src/jclient.c`

Integration policy:

- Reference-only for now. Do not link `dspc` directly into `mamut-engine`.
- Do not add FFTW, libsndfile, JACK, or AVX2 requirements to the standalone
  synth runtime as a side effect of studying this code.
- If an algorithm is useful, port the small formula or offline tool path into
  Rust with explicit tests and a separate license review.
- Keep realtime-path changes allocation-free and blocking-free; `dspc` is not a
  shortcut around the existing Mamut realtime boundary.

Candidate Mamut uses:

- Compare PEQ and Butterworth/Linkwitz-Riley formulas against future Mamut
  filter-color or output-shaping work.
- Use Remez/FIR material as an offline design reference, not as a live dependency.
- Study vector loop shapes for offline benches before considering any SIMD work
  in the Rust DSP layer.
