use crate::safety::sanitize_sample;

#[derive(Debug, Clone)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_index: usize,
}

impl DelayLine {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples.max(2)],
            write_index: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.write_index = 0;
    }

    pub fn push(&mut self, input: f32) {
        self.buffer[self.write_index] = sanitize_sample(input);
        self.write_index = (self.write_index + 1) % self.buffer.len();
    }

    pub fn read_linear(&self, delay_samples: f32) -> f32 {
        let len = self.buffer.len();
        let delay_samples = finite_or(delay_samples, 1.0).clamp(1.0, (len - 1) as f32);
        let read_pos = (self.write_index + len) as f32 - delay_samples;
        let read_floor = read_pos.floor();
        let frac = read_pos - read_floor;
        let index_a = (read_floor as usize) % len;
        let index_b = (index_a + 1) % len;
        self.buffer[index_a] * (1.0 - frac) + self.buffer[index_b] * frac
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnePoleDamping {
    state: f32,
}

impl OnePoleDamping {
    pub fn new() -> Self {
        Self { state: 0.0 }
    }

    pub fn reset(&mut self) {
        self.state = 0.0;
    }

    pub fn process(&mut self, input: f32, damping: f32) -> f32 {
        let damping = finite_or(damping, 0.0).clamp(0.0, 0.99);
        self.state += (sanitize_sample(input) - self.state) * (1.0 - damping);
        sanitize_sample(self.state)
    }
}

impl Default for OnePoleDamping {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CombFilter {
    delay: DelayLine,
    damping: OnePoleDamping,
}

impl CombFilter {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            delay: DelayLine::new(max_delay_samples),
            damping: OnePoleDamping::new(),
        }
    }

    pub fn clear(&mut self) {
        self.delay.clear();
        self.damping.reset();
    }

    pub fn process(&mut self, input: f32, delay_samples: f32, feedback: f32, damping: f32) -> f32 {
        let feedback = finite_or(feedback, 0.0).clamp(-0.98, 0.98);
        let delayed = self.delay.read_linear(delay_samples);
        let damped = self.damping.process(delayed, damping);
        self.delay.push(input + damped * feedback);
        sanitize_sample(delayed)
    }
}

#[derive(Debug, Clone)]
pub struct AllpassFilter {
    delay: DelayLine,
}

impl AllpassFilter {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            delay: DelayLine::new(max_delay_samples),
        }
    }

    pub fn clear(&mut self) {
        self.delay.clear();
    }

    pub fn process(&mut self, input: f32, delay_samples: f32, feedback: f32) -> f32 {
        let input = sanitize_sample(input);
        let feedback = finite_or(feedback, 0.0).clamp(-0.98, 0.98);
        let delayed = self.delay.read_linear(delay_samples);
        let output = delayed - input * feedback;
        self.delay.push(input + output * feedback);
        sanitize_sample(output)
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
