#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseRng {
    state: u32,
}

impl NoiseRng {
    pub fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_bipolar(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let normalized = (self.state as f32 / u32::MAX as f32).clamp(0.0, 1.0);
        normalized * 2.0 - 1.0
    }
}

pub fn color_noise_sample(raw: f32, color_index: f32, state: &mut f32) -> f32 {
    match color_index.round() as i32 {
        1 => {
            *state = *state * 0.965 + raw * 0.035;
            (*state * 3.1).clamp(-1.0, 1.0)
        }
        2 => {
            *state = *state * 0.920 + raw * 0.080;
            (*state * 2.0).clamp(-1.0, 1.0)
        }
        3 => {
            let bright = raw - *state * 0.55;
            *state = raw;
            bright.clamp(-1.0, 1.0)
        }
        _ => raw,
    }
}
