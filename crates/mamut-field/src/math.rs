pub(crate) fn seed_to_state(seed: u64) -> u64 {
    let state = seed ^ 0x9E37_79B9_7F4A_7C15;
    if state == 0 {
        0xA076_1D64_78BD_642F
    } else {
        state
    }
}

pub(crate) fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state ^ (*state >> 33)
}

pub(crate) fn next_unit(state: &mut u64) -> f32 {
    let bits = (next_u64(state) >> 40) as u32;
    bits as f32 / 0x00FF_FFFF_u32 as f32
}

pub(crate) fn next_bipolar(state: &mut u64) -> f32 {
    next_unit(state) * 2.0 - 1.0
}

pub(crate) fn wrap_index(index: isize, len: usize) -> usize {
    let len = len as isize;
    index.rem_euclid(len) as usize
}

pub(crate) fn toroidal_distance(a: usize, b: usize, len: usize) -> usize {
    let direct = a.abs_diff(b);
    direct.min(len - direct)
}

pub(crate) fn wrap_phase(phase: f32) -> f32 {
    phase - phase.floor()
}

pub(crate) fn sanitize_positive(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

pub(crate) fn sanitize_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

pub(crate) fn sanitize_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

pub(crate) fn soft_limit(sample: f32) -> f32 {
    (sample * 1.22).tanh().clamp(-1.0, 1.0)
}

pub(crate) fn fast_sin01(phase: f32) -> f32 {
    let phase = wrap_phase(phase);
    let triangle = if phase < 0.5 {
        phase * 4.0 - 1.0
    } else {
        3.0 - phase * 4.0
    };
    triangle * (1.5 - 0.5 * triangle.abs())
}

pub(crate) fn fast_cos01(phase: f32) -> f32 {
    fast_sin01(phase + 0.25)
}
