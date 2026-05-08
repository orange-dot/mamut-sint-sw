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
