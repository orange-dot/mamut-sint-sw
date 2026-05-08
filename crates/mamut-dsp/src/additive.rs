pub fn additive_partial_count(partial_count: f32) -> usize {
    (partial_count.round() as usize).clamp(4, 8)
}

pub fn additive_ratio(partial_index: usize, harmonic_spread: f32, inharmonicity: f32) -> f32 {
    let harmonic = partial_index as f32 + 1.0;
    let higher = partial_index as f32;
    let spread = harmonic_spread.clamp(0.0, 1.0) * higher * 0.055;
    let inharmonic = inharmonicity.clamp(0.0, 1.0) * higher * higher * 0.018;
    (harmonic + spread + inharmonic).max(1.0)
}

pub fn additive_weight(
    partial_index: usize,
    partial_count: usize,
    odd_even_balance: f32,
    spectral_tilt: f32,
) -> f32 {
    let harmonic = partial_index + 1;
    let base = 1.0 / (harmonic as f32).sqrt();
    let parity_sign = if harmonic % 2 == 1 { 1.0 } else { -1.0 };
    let parity = (1.0 + odd_even_balance.clamp(-1.0, 1.0) * parity_sign * 0.65).clamp(0.05, 1.65);
    let high_position = if partial_count <= 1 {
        0.0
    } else {
        partial_index as f32 / (partial_count - 1) as f32
    };
    let tilt = 2.0_f32.powf(spectral_tilt.clamp(-1.0, 1.0) * (high_position - 0.25) * 1.4);
    (base * parity * tilt).max(0.0)
}
