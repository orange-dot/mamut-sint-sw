use super::*;

/// Raw control-domain targets for the five Mozaik session controls. These
/// are the values `reset_controllers` restores and the snapshot reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MozaikControlValues {
    pub(crate) mix: f32,
    pub(crate) slope: f32,
    pub(crate) contrast: f32,
    pub(crate) phason: f32,
    pub(crate) drift: f32,
}

impl MozaikControlValues {
    /// The on-enable defaults (also the `reset_controllers` restore point):
    /// audible mix, slope at the golden detent, contrast at `gamma = tau`,
    /// phason zero, drift frozen.
    pub(crate) fn defaults() -> Self {
        Self {
            mix: MOZAIK_DEFAULT_MIX,
            slope: MOZAIK_DEFAULT_SLOPE_CONTROL,
            contrast: MOZAIK_DEFAULT_CONTRAST_CONTROL,
            phason: MOZAIK_DEFAULT_PHASON,
            drift: MOZAIK_DEFAULT_DRIFT,
        }
    }

    pub(crate) fn disabled() -> Self {
        Self {
            mix: 0.0,
            slope: MOZAIK_DEFAULT_SLOPE_CONTROL,
            contrast: MOZAIK_DEFAULT_CONTRAST_CONTROL,
            phason: MOZAIK_DEFAULT_PHASON,
            drift: MOZAIK_DEFAULT_DRIFT,
        }
    }
}

/// Per-frame Mozaik values pushed into the voice loop; also the values a
/// voice trigger configures its oscillator from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MozaikFrame {
    pub(crate) mix: f32,
    pub(crate) slope_q32: u32,
    pub(crate) contrast: f32,
    pub(crate) phason_q32: u32,
}

impl MozaikFrame {
    pub(crate) fn idle() -> Self {
        Self {
            mix: 0.0,
            slope_q32: mamut_dsp::MOZAIK_SLOPE_GOLDEN_Q32,
            contrast: mamut_dsp::MOZAIK_DEFAULT_CONTRAST,
            phason_q32: 0,
        }
    }
}

impl Engine {
    pub fn set_mozaik_mode(&mut self, mode: MozaikMode) -> MozaikSnapshot {
        self.mozaik_mode = mode;
        self.mozaik_drift_accum = 0.0;
        match mode {
            MozaikMode::Disabled => {
                self.mozaik_controls = MozaikControlValues::disabled();
                self.reinit_mozaik_smoothing_to_targets();
            }
            MozaikMode::Enabled { .. } => {
                self.mozaik_controls = MozaikControlValues::defaults();
                self.reinit_mozaik_smoothing_to_targets();
                // Click-free enable: the mix alone ramps from silence up to
                // the default; slope/contrast/phason start at their targets.
                self.mozaik_mix = LinearSmoother::new(0.0);
                self.mozaik_mix.set_target(
                    self.mozaik_controls.mix,
                    smoothing_sample_count(self.config.sample_rate_hz, MOZAIK_MIX_ATTACK_MS),
                );
                // Keep the cached frame consistent with the ramp start; the
                // first rendered frame overwrites it anyway.
                self.mozaik_frame.mix = self.mozaik_mix.current();
                for index in 0..self.voices.len() {
                    self.configure_mozaik_voice_on_trigger(index);
                }
            }
        }
        self.mozaik_snapshot()
    }

    pub fn set_mozaik_param(&mut self, param: MozaikParam, value: f32) -> MozaikSnapshot {
        let smoothing =
            smoothing_sample_count(self.config.sample_rate_hz, MOZAIK_CONTROL_SMOOTHING_MS);
        match param {
            MozaikParam::Mix => {
                let value = sanitize_mozaik_control(value, MOZAIK_DEFAULT_MIX);
                self.mozaik_controls.mix = value;
                let mix_ms = if value > self.mozaik_mix.current() {
                    MOZAIK_MIX_ATTACK_MS
                } else {
                    MOZAIK_MIX_RELEASE_MS
                };
                self.mozaik_mix.set_target(
                    value,
                    smoothing_sample_count(self.config.sample_rate_hz, mix_ms),
                );
            }
            MozaikParam::Slope => {
                let value = sanitize_mozaik_control(value, MOZAIK_DEFAULT_SLOPE_CONTROL);
                self.mozaik_controls.slope = value;
                let (sigma, snapped) = mozaik_sigma_from_control(value);
                self.mozaik_slope_snapped = snapped;
                self.mozaik_slope_sigma.set_target(sigma, smoothing);
            }
            MozaikParam::Contrast => {
                let value = sanitize_mozaik_control(value, MOZAIK_DEFAULT_CONTRAST_CONTROL);
                self.mozaik_controls.contrast = value;
                self.mozaik_contrast_gamma
                    .set_target(mozaik_gamma_from_control(value), smoothing);
            }
            MozaikParam::Phason => {
                let value = sanitize_mozaik_control(value, MOZAIK_DEFAULT_PHASON);
                self.mozaik_controls.phason = value;
                self.mozaik_phason.set_target(value, smoothing);
            }
            MozaikParam::Drift => {
                let value = sanitize_mozaik_control(value, MOZAIK_DEFAULT_DRIFT);
                self.mozaik_controls.drift = value;
                self.mozaik_drift.set_target(value, smoothing);
            }
        }
        self.mozaik_snapshot()
    }

    pub const fn mozaik_mode(&self) -> MozaikMode {
        self.mozaik_mode
    }

    pub(super) fn mozaik_snapshot(&self) -> MozaikSnapshot {
        let (slope_sigma, _) = mozaik_sigma_from_control(self.mozaik_controls.slope);
        MozaikSnapshot {
            mode: self.mozaik_mode,
            mix: self.mozaik_controls.mix,
            effective_mix: self.mozaik_mix.current(),
            slope: self.mozaik_controls.slope,
            slope_sigma,
            slope_snapped: self.mozaik_slope_snapped,
            contrast: self.mozaik_controls.contrast,
            contrast_gamma: mozaik_gamma_from_control(self.mozaik_controls.contrast),
            phason: self.mozaik_controls.phason,
            drift: self.mozaik_controls.drift,
            drift_phason: self.mozaik_drift_accum as f32,
        }
    }

    /// `reset_controllers` restore point: back to the on-enable defaults,
    /// smoothed from the current values. No-op while disabled.
    pub(super) fn reset_mozaik_controls_to_defaults(&mut self) {
        if !matches!(self.mozaik_mode, MozaikMode::Enabled { .. }) {
            return;
        }
        self.mozaik_controls = MozaikControlValues::defaults();
        self.mozaik_drift_accum = 0.0;
        let smoothing =
            smoothing_sample_count(self.config.sample_rate_hz, MOZAIK_CONTROL_SMOOTHING_MS);
        let mix_ms = if self.mozaik_controls.mix > self.mozaik_mix.current() {
            MOZAIK_MIX_ATTACK_MS
        } else {
            MOZAIK_MIX_RELEASE_MS
        };
        self.mozaik_mix.set_target(
            self.mozaik_controls.mix,
            smoothing_sample_count(self.config.sample_rate_hz, mix_ms),
        );
        let (sigma, snapped) = mozaik_sigma_from_control(self.mozaik_controls.slope);
        self.mozaik_slope_snapped = snapped;
        self.mozaik_slope_sigma.set_target(sigma, smoothing);
        self.mozaik_contrast_gamma.set_target(
            mozaik_gamma_from_control(self.mozaik_controls.contrast),
            smoothing,
        );
        self.mozaik_phason
            .set_target(self.mozaik_controls.phason, smoothing);
        self.mozaik_drift
            .set_target(self.mozaik_controls.drift, smoothing);
    }

    /// Jump every Mozaik smoother straight to its control target and refresh
    /// the cached frame values. Used on mode changes and runtime resets,
    /// where no audio is flowing that a jump could click.
    pub(super) fn reinit_mozaik_smoothing_to_targets(&mut self) {
        let (sigma, snapped) = mozaik_sigma_from_control(self.mozaik_controls.slope);
        self.mozaik_slope_snapped = snapped;
        self.mozaik_mix = LinearSmoother::new(self.mozaik_controls.mix);
        self.mozaik_slope_sigma = LinearSmoother::new(sigma);
        self.mozaik_contrast_gamma =
            LinearSmoother::new(mozaik_gamma_from_control(self.mozaik_controls.contrast));
        self.mozaik_phason = LinearSmoother::new(self.mozaik_controls.phason);
        self.mozaik_drift = LinearSmoother::new(self.mozaik_controls.drift);
        self.mozaik_frame = MozaikFrame {
            mix: self.mozaik_mix.current(),
            slope_q32: mozaik_slope_q32_from_sigma(sigma),
            contrast: self.mozaik_contrast_gamma.current(),
            phason_q32: mozaik_phason_offset_q32(
                self.mozaik_phason.current(),
                self.mozaik_drift_accum,
            ),
        };
    }

    /// Advance the per-frame Mozaik control state. Returns `None` while
    /// disabled so the voice loop skips the source entirely and the disabled
    /// render stays bit-identical to the pre-slice baseline.
    pub(super) fn advance_mozaik_frame(&mut self) -> Option<MozaikFrame> {
        if !matches!(self.mozaik_mode, MozaikMode::Enabled { .. }) {
            return None;
        }
        let mix = self.mozaik_mix.next_value();
        let sigma = self.mozaik_slope_sigma.next_value();
        let gamma = self.mozaik_contrast_gamma.next_value();
        let phason = self.mozaik_phason.next_value();
        let drift = self.mozaik_drift.next_value();
        let drift_rate = MOZAIK_MAX_DRIFT_CYCLES_PER_SECOND * drift * drift;
        if drift_rate > 0.0 {
            self.mozaik_drift_accum = (self.mozaik_drift_accum
                + f64::from(drift_rate) / f64::from(self.config.sample_rate_hz))
            .fract();
        }
        let frame = MozaikFrame {
            mix,
            slope_q32: mozaik_slope_q32_from_sigma(sigma),
            contrast: gamma,
            phason_q32: mozaik_phason_offset_q32(phason, self.mozaik_drift_accum),
        };
        self.mozaik_frame = frame;
        Some(frame)
    }

    /// Reseat a voice's oscillator from the layer seed and current control
    /// frame. Called on voice trigger and when the layer is enabled over
    /// already-sounding voices.
    pub(super) fn configure_mozaik_voice_on_trigger(&mut self, index: usize) {
        let MozaikMode::Enabled { seed } = self.mozaik_mode else {
            return;
        };
        let base = mozaik_voice_phason_q32(seed, index);
        let frame = self.mozaik_frame;
        let Some(voice) = self.voices.get_mut(index) else {
            return;
        };
        voice.mozaik_phason_base_q32 = base;
        voice
            .mozaik
            .reset_to_phason_q32(base.wrapping_add(frame.phason_q32));
        voice.mozaik.set_slope_q32(frame.slope_q32);
        voice.mozaik.set_contrast(frame.contrast);
    }
}

fn mozaik_phason_offset_q32(phason_control: f32, drift_accum: f64) -> u32 {
    let offset = (f64::from(phason_control) + drift_accum).rem_euclid(1.0);
    (offset * 4_294_967_296.0) as u64 as u32
}
