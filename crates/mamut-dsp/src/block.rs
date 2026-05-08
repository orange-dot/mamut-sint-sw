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
