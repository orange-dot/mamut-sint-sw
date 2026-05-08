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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrEnvelope {
    timing: AdsrTiming,
    sample_rate_hz: f32,
    level: f32,
    stage: EnvelopeStage,
}

impl AdsrEnvelope {
    pub fn new(sample_rate_hz: f32, timing: AdsrTiming) -> Self {
        Self {
            timing: timing.clamp(),
            sample_rate_hz: sample_rate_hz.max(1.0),
            level: 0.0,
            stage: EnvelopeStage::Idle,
        }
    }

    pub fn set_timing(&mut self, timing: AdsrTiming) {
        self.timing = timing.clamp();
    }

    pub fn note_on(&mut self) {
        self.stage = EnvelopeStage::Attack;
    }

    pub fn note_off(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Release;
        }
    }

    pub fn is_idle(&self) -> bool {
        self.stage == EnvelopeStage::Idle
    }

    pub fn current_level(&self) -> f32 {
        self.level
    }

    pub fn stage(&self) -> EnvelopeStage {
        self.stage
    }

    pub fn next_sample(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => {
                self.level = 0.0;
            }
            EnvelopeStage::Attack => {
                let attack_samples = (self.timing.attack_ms * 0.001 * self.sample_rate_hz).max(1.0);
                self.level += 1.0 / attack_samples;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                }
            }
            EnvelopeStage::Decay => {
                let decay_samples = (self.timing.decay_ms * 0.001 * self.sample_rate_hz).max(1.0);
                self.level += (self.timing.sustain - self.level) / decay_samples;
                if (self.level - self.timing.sustain).abs() < 0.001 {
                    self.level = self.timing.sustain;
                    self.stage = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => {
                self.level = self.timing.sustain;
            }
            EnvelopeStage::Release => {
                let release_samples =
                    (self.timing.release_ms * 0.001 * self.sample_rate_hz).max(1.0);
                self.level += (0.0 - self.level) / release_samples;
                if self.level.abs() < 0.0005 {
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                }
            }
        }

        self.level.clamp(0.0, 1.0)
    }
}
