#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrTiming {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
}

impl AdsrTiming {
    pub fn clamp(self) -> Self {
        Self {
            attack_ms: self.attack_ms.max(1.0),
            decay_ms: self.decay_ms.max(1.0),
            sustain: self.sustain.clamp(0.0, 1.0),
            release_ms: self.release_ms.max(1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearSmoother {
    current: f32,
    target: f32,
    step: f32,
    samples_remaining: usize,
}

impl LinearSmoother {
    pub fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            step: 0.0,
            samples_remaining: 0,
        }
    }

    pub fn current(&self) -> f32 {
        self.current
    }

    pub fn set_target(&mut self, target: f32, sample_count: usize) {
        self.target = target;
        self.samples_remaining = sample_count;
        self.step = if sample_count == 0 {
            self.current = target;
            0.0
        } else {
            (target - self.current) / sample_count as f32
        };
    }

    pub fn next_value(&mut self) -> f32 {
        if self.samples_remaining > 0 {
            self.current += self.step;
            self.samples_remaining -= 1;
            if self.samples_remaining == 0 {
                self.current = self.target;
            }
        }

        self.current
    }
}

#[derive(Debug)]
pub struct StereoBlockMut<'a> {
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
}

impl<'a> StereoBlockMut<'a> {
    pub fn new(left: &'a mut [f32], right: &'a mut [f32]) -> Self {
        Self { left, right }
    }

    pub fn frames(&self) -> usize {
        self.left.len().min(self.right.len())
    }

    pub fn clear(&mut self) {
        let frames = self.frames();
        self.left[..frames].fill(0.0);
        self.right[..frames].fill(0.0);
    }

    pub fn peak_abs(&self) -> f32 {
        self.left
            .iter()
            .zip(self.right.iter())
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0, f32::max)
    }
}

pub fn sanitize_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

pub fn sanitize_block(block: &mut StereoBlockMut<'_>) {
    let frames = block.frames();
    for index in 0..frames {
        block.left[index] = sanitize_sample(block.left[index]);
        block.right[index] = sanitize_sample(block.right[index]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_smoother_reaches_target() {
        let mut smoother = LinearSmoother::new(0.0);
        smoother.set_target(1.0, 4);

        for _ in 0..4 {
            smoother.next_value();
        }

        assert_eq!(smoother.current(), 1.0);
    }

    #[test]
    fn sanitize_non_finite_samples() {
        assert_eq!(sanitize_sample(f32::NAN), 0.0);
        assert_eq!(sanitize_sample(f32::INFINITY), 0.0);
    }
}
