#[derive(Debug)]
pub struct MonoBlockMut<'a> {
    pub samples: &'a mut [f32],
}

impl<'a> MonoBlockMut<'a> {
    pub fn new(samples: &'a mut [f32]) -> Self {
        Self { samples }
    }

    pub fn frames(&self) -> usize {
        self.samples.len()
    }

    pub fn clear(&mut self) {
        self.samples.fill(0.0);
    }

    pub fn apply_gain(&mut self, gain: f32) {
        for sample in self.samples.iter_mut() {
            *sample *= gain;
        }
    }

    pub fn mix_from(&mut self, source: &[f32], gain: f32) {
        let frames = self.frames().min(source.len());
        for (index, sample) in source.iter().take(frames).enumerate() {
            self.samples[index] += *sample * gain;
        }
    }

    pub fn peak_abs(&self) -> f32 {
        let mut peak = 0.0_f32;
        for sample in self.samples.iter() {
            peak = peak.max(sample.abs());
        }
        peak
    }

    pub fn rms(&self) -> f32 {
        let frames = self.frames();
        if frames == 0 {
            return 0.0;
        }

        let mut sum_squares = 0.0_f32;
        for sample in self.samples.iter() {
            sum_squares += sample * sample;
        }
        (sum_squares / frames as f32).sqrt()
    }

    pub fn dc_offset(&self) -> f32 {
        let frames = self.frames();
        if frames == 0 {
            return 0.0;
        }

        let mut sum = 0.0_f32;
        for sample in self.samples.iter() {
            sum += *sample;
        }
        sum / frames as f32
    }

    pub fn is_finite(&self) -> bool {
        self.samples.iter().all(|sample| sample.is_finite())
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

    pub fn apply_gain(&mut self, gain: f32) {
        let frames = self.frames();
        for index in 0..frames {
            self.left[index] *= gain;
            self.right[index] *= gain;
        }
    }

    pub fn mix_from(&mut self, left: &[f32], right: &[f32], gain: f32) {
        let frames = self.frames().min(left.len()).min(right.len());
        for index in 0..frames {
            self.left[index] += left[index] * gain;
            self.right[index] += right[index] * gain;
        }
    }

    pub fn peak_abs(&self) -> f32 {
        let frames = self.frames();
        let mut peak = 0.0_f32;
        for index in 0..frames {
            peak = peak.max(self.left[index].abs().max(self.right[index].abs()));
        }
        peak
    }

    pub fn rms(&self) -> f32 {
        let frames = self.frames();
        if frames == 0 {
            return 0.0;
        }

        let mut sum_squares = 0.0_f32;
        for index in 0..frames {
            sum_squares += self.left[index] * self.left[index];
            sum_squares += self.right[index] * self.right[index];
        }
        (sum_squares / (frames * 2) as f32).sqrt()
    }

    pub fn dc_offset(&self) -> (f32, f32) {
        let frames = self.frames();
        if frames == 0 {
            return (0.0, 0.0);
        }

        let mut left_sum = 0.0_f32;
        let mut right_sum = 0.0_f32;
        for index in 0..frames {
            left_sum += self.left[index];
            right_sum += self.right[index];
        }
        (left_sum / frames as f32, right_sum / frames as f32)
    }

    pub fn is_finite(&self) -> bool {
        let frames = self.frames();
        for index in 0..frames {
            if !self.left[index].is_finite() || !self.right[index].is_finite() {
                return false;
            }
        }
        true
    }
}
