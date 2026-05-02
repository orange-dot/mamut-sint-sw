use std::f32::consts::TAU;
use std::ops::Range;

pub const BCS_V0_1_SAMPLE_RATE_HZ: u32 = 48_000;
pub const BCS_V0_1_OVERSAMPLE: usize = 4;
pub const BCS_V0_1_DURATION_SECONDS: usize = 8;
pub const BCS_V0_1_BASE_NOTE: u8 = 48;
pub const BCS_V0_1_BASE_FREQUENCY_HZ: f32 = 130.812_78;

const BCS_V0_1_STATE_CEILING: f32 = 32.0;
const LYAPUNOV_PERTURBATION: f32 = 1.0e-5;
const LYAPUNOV_INTERVAL_FRAMES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BcsScenario {
    StableAnchor,
    EdgeSweep,
    SubharmonicPressure,
    RecoveryReturn,
}

impl BcsScenario {
    pub const ALL: [Self; 4] = [
        Self::StableAnchor,
        Self::EdgeSweep,
        Self::SubharmonicPressure,
        Self::RecoveryReturn,
    ];

    pub const fn wav_file_name(self) -> &'static str {
        match self {
            Self::StableAnchor => "bcs_v0_1_stable_anchor.wav",
            Self::EdgeSweep => "bcs_v0_1_edge_sweep.wav",
            Self::SubharmonicPressure => "bcs_v0_1_subharmonic_pressure.wav",
            Self::RecoveryReturn => "bcs_v0_1_recovery_return.wav",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BcsParams {
    pub sample_rate_hz: f32,
    pub base_frequency_hz: f32,
    pub hopf_mu: f32,
    pub hopf_nonlinearity: f32,
    pub duffing_frequency_ratio: f32,
    pub duffing_damping: f32,
    pub duffing_edge: f32,
    pub duffing_cubic_stiffness: f32,
    pub duffing_cubic_damping: f32,
    pub duffing_drive: f32,
    pub duffing_feedback: f32,
    pub hopf_mix: f32,
    pub duffing_mix: f32,
    pub output_gain: f32,
    pub state_ceiling: f32,
}

impl BcsParams {
    pub const fn stable_anchor() -> Self {
        Self {
            sample_rate_hz: BCS_V0_1_SAMPLE_RATE_HZ as f32,
            base_frequency_hz: BCS_V0_1_BASE_FREQUENCY_HZ,
            hopf_mu: 18.0,
            hopf_nonlinearity: 32.0,
            duffing_frequency_ratio: 1.0,
            duffing_damping: 0.18,
            duffing_edge: 0.10,
            duffing_cubic_stiffness: 0.35,
            duffing_cubic_damping: 1.15,
            duffing_drive: 0.030,
            duffing_feedback: 0.004,
            hopf_mix: 1.00,
            duffing_mix: 0.16,
            output_gain: 0.58,
            state_ceiling: BCS_V0_1_STATE_CEILING,
        }
    }

    pub const fn edge_sweep() -> Self {
        Self {
            duffing_edge: 0.42,
            duffing_drive: 0.105,
            duffing_feedback: 0.010,
            duffing_mix: 0.40,
            output_gain: 0.54,
            ..Self::stable_anchor()
        }
    }

    pub const fn subharmonic_pressure() -> Self {
        Self {
            hopf_mu: 16.0,
            duffing_frequency_ratio: 0.42,
            duffing_damping: 0.10,
            duffing_edge: 0.40,
            duffing_cubic_stiffness: 0.48,
            duffing_cubic_damping: 1.40,
            duffing_drive: 0.050,
            duffing_feedback: 0.002,
            hopf_mix: 0.05,
            duffing_mix: 1.35,
            output_gain: 0.60,
            ..Self::stable_anchor()
        }
    }

    pub const fn recovery_return() -> Self {
        Self {
            duffing_edge: 0.36,
            duffing_drive: 0.090,
            duffing_feedback: 0.008,
            duffing_mix: 0.36,
            output_gain: 0.56,
            ..Self::stable_anchor()
        }
    }

    pub const fn for_scenario(scenario: BcsScenario) -> Self {
        match scenario {
            BcsScenario::StableAnchor => Self::stable_anchor(),
            BcsScenario::EdgeSweep => Self::edge_sweep(),
            BcsScenario::SubharmonicPressure => Self::subharmonic_pressure(),
            BcsScenario::RecoveryReturn => Self::recovery_return(),
        }
    }

    fn sanitized(self) -> Self {
        Self {
            sample_rate_hz: sanitize_positive(self.sample_rate_hz, BCS_V0_1_SAMPLE_RATE_HZ as f32),
            base_frequency_hz: sanitize_positive(
                self.base_frequency_hz,
                BCS_V0_1_BASE_FREQUENCY_HZ,
            ),
            hopf_mu: sanitize_f32(self.hopf_mu, 18.0),
            hopf_nonlinearity: sanitize_positive(self.hopf_nonlinearity, 32.0),
            duffing_frequency_ratio: sanitize_positive(self.duffing_frequency_ratio, 1.0),
            duffing_damping: sanitize_f32(self.duffing_damping, 0.18),
            duffing_edge: sanitize_f32(self.duffing_edge, 0.10),
            duffing_cubic_stiffness: sanitize_f32(self.duffing_cubic_stiffness, 0.35),
            duffing_cubic_damping: sanitize_f32(self.duffing_cubic_damping, 1.15),
            duffing_drive: sanitize_f32(self.duffing_drive, 0.030),
            duffing_feedback: sanitize_f32(self.duffing_feedback, 0.004),
            hopf_mix: sanitize_f32(self.hopf_mix, 1.00),
            duffing_mix: sanitize_f32(self.duffing_mix, 0.16),
            output_gain: sanitize_f32(self.output_gain, 0.58),
            state_ceiling: sanitize_positive(self.state_ceiling, BCS_V0_1_STATE_CEILING),
        }
    }
}

impl Default for BcsParams {
    fn default() -> Self {
        Self::stable_anchor()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BcsGesture {
    pub scenario: BcsScenario,
}

impl BcsGesture {
    pub const fn new(scenario: BcsScenario) -> Self {
        Self { scenario }
    }

    pub const fn v0_1(scenario: BcsScenario) -> Self {
        Self::new(scenario)
    }

    pub const fn duration_seconds(self) -> usize {
        BCS_V0_1_DURATION_SECONDS
    }

    pub fn frames(self, sample_rate_hz: u32) -> usize {
        (sample_rate_hz as usize).saturating_mul(self.duration_seconds())
    }

    fn control_at_frame(self, frame: usize, sample_rate_hz: f32) -> BcsControl {
        let seconds =
            frame as f32 / sanitize_positive(sample_rate_hz, BCS_V0_1_SAMPLE_RATE_HZ as f32);
        match self.scenario {
            BcsScenario::StableAnchor => BcsControl {
                drive: 0.32,
                edge: 0.18,
                duffing_mix: 0.70,
                output_gain: 1.0,
                hopf_mu_bias: 0.0,
                duffing_frequency_scale: 1.0,
            },
            BcsScenario::EdgeSweep => {
                let ramp = smoothstep((seconds / 7.2).clamp(0.0, 1.0));
                BcsControl {
                    drive: 0.38 + ramp * 1.35,
                    edge: 0.18 + ramp * 1.45,
                    duffing_mix: 0.74 + ramp * 0.92,
                    output_gain: 0.92,
                    hopf_mu_bias: ramp * 2.0,
                    duffing_frequency_scale: 1.0 + ramp * 0.025,
                }
            }
            BcsScenario::SubharmonicPressure => {
                let attack = smoothstep((seconds / 1.15).clamp(0.0, 1.0));
                BcsControl {
                    drive: 0.42 + attack * 0.38,
                    edge: 0.85 + attack * 0.45,
                    duffing_mix: 1.0,
                    output_gain: 0.96,
                    hopf_mu_bias: -1.0 * attack,
                    duffing_frequency_scale: 1.0,
                }
            }
            BcsScenario::RecoveryReturn => {
                let burst = trapezoid(seconds, 2.10, 0.55, 1.35, 1.00);
                let settling = if seconds < 4.95 {
                    1.0
                } else {
                    1.0 - smoothstep(((seconds - 4.95) / 1.15).clamp(0.0, 1.0))
                };
                BcsControl {
                    drive: 0.32 + burst * 1.35 * settling,
                    edge: 0.18 + burst * 1.58 * settling,
                    duffing_mix: 0.70 + burst * 0.95 * settling,
                    output_gain: 1.0,
                    hopf_mu_bias: burst * 1.5 * settling,
                    duffing_frequency_scale: 1.0 + burst * 0.02,
                }
            }
        }
    }

    fn analysis_window(self, frames: usize, sample_rate_hz: u32) -> Range<usize> {
        let rate = sample_rate_hz as usize;
        let start_seconds: usize = match self.scenario {
            BcsScenario::StableAnchor => 2,
            BcsScenario::EdgeSweep => 4,
            BcsScenario::SubharmonicPressure => 3,
            BcsScenario::RecoveryReturn => 6,
        };
        let start = start_seconds.saturating_mul(rate).min(frames);
        start..frames
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BcsDiagnostics {
    pub frames: usize,
    pub rms: f32,
    pub peak_abs: f32,
    pub finite: bool,
    pub max_state_abs: f32,
    pub unsafe_state: bool,
    pub estimated_period_hz: Option<f32>,
    pub subharmonic_ratio: Option<f32>,
    pub lyapunov_proxy: f32,
    pub unsafe_events: u64,
    pub zero_crossings: usize,
    pub period_estimate_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BcsVoice {
    params: BcsParams,
    state: BcsState,
    frame_index: usize,
    max_state_abs: f32,
    unsafe_events: u64,
    unsafe_state: bool,
}

impl BcsVoice {
    pub fn new(params: BcsParams) -> Self {
        let params = params.sanitized();
        let state = BcsState::seeded();
        Self {
            params,
            state,
            frame_index: 0,
            max_state_abs: state.max_abs(),
            unsafe_events: 0,
            unsafe_state: false,
        }
    }

    pub fn for_scenario(scenario: BcsScenario) -> Self {
        Self::new(BcsParams::for_scenario(scenario))
    }

    pub fn reset(&mut self) {
        self.state = BcsState::seeded();
        self.frame_index = 0;
        self.max_state_abs = self.state.max_abs();
        self.unsafe_events = 0;
        self.unsafe_state = false;
    }

    pub fn params(&self) -> BcsParams {
        self.params
    }

    pub fn set_base_frequency_hz(&mut self, frequency_hz: f32) {
        self.params.base_frequency_hz = sanitize_positive(frequency_hz, BCS_V0_1_BASE_FREQUENCY_HZ);
    }

    pub fn next_sample(&mut self, gesture: BcsGesture) -> f32 {
        let params = self.params.sanitized();
        let control = gesture.control_at_frame(self.frame_index, params.sample_rate_hz);
        let dt = 1.0 / (params.sample_rate_hz * BCS_V0_1_OVERSAMPLE as f32);

        for _ in 0..BCS_V0_1_OVERSAMPLE {
            self.state = rk4_step(self.state, params, control, dt);
            self.update_safety();
        }

        self.frame_index = self.frame_index.saturating_add(1);
        let sample = readout(self.state, params, control);
        if sample.is_finite() {
            sample
        } else {
            self.unsafe_state = true;
            self.unsafe_events = self.unsafe_events.saturating_add(1);
            0.0
        }
    }

    pub fn max_state_abs(&self) -> f32 {
        self.max_state_abs
    }

    pub fn unsafe_events(&self) -> u64 {
        self.unsafe_events
    }

    pub fn unsafe_state(&self) -> bool {
        self.unsafe_state
    }

    fn update_safety(&mut self) {
        let max_abs = self.state.max_abs();
        if max_abs.is_finite() {
            self.max_state_abs = self.max_state_abs.max(max_abs);
        }

        if !self.state.is_finite() || max_abs > self.params.state_ceiling {
            self.unsafe_state = true;
            self.unsafe_events = self.unsafe_events.saturating_add(1);
        }
    }
}

pub fn render_scenario(
    scenario: BcsScenario,
) -> Result<(Vec<f32>, BcsDiagnostics), BcsDiagnostics> {
    render_with_params(BcsParams::for_scenario(scenario), BcsGesture::new(scenario))
}

pub fn render_with_params(
    params: BcsParams,
    gesture: BcsGesture,
) -> Result<(Vec<f32>, BcsDiagnostics), BcsDiagnostics> {
    let params = params.sanitized();
    let sample_rate_hz = params.sample_rate_hz.round().max(1.0) as u32;
    let frames = gesture.frames(sample_rate_hz);
    let mut voice = BcsVoice::new(params);
    let mut samples = Vec::with_capacity(frames);
    let mut shadow = BcsState {
        hopf_re: voice.state.hopf_re + LYAPUNOV_PERTURBATION,
        ..voice.state
    };
    let mut lyapunov_sum = 0.0_f32;
    let mut lyapunov_count = 0_u32;
    let mut shadow_unsafe = false;
    let mut shadow_unsafe_events = 0_u64;

    for frame in 0..frames {
        let sample = voice.next_sample(gesture);
        samples.push(sample);

        let control = gesture.control_at_frame(frame, params.sample_rate_hz);
        let dt = 1.0 / (params.sample_rate_hz * BCS_V0_1_OVERSAMPLE as f32);
        for _ in 0..BCS_V0_1_OVERSAMPLE {
            shadow = rk4_step(shadow, params, control, dt);
            if !shadow.is_finite() || shadow.max_abs() > params.state_ceiling {
                shadow_unsafe = true;
                shadow_unsafe_events = shadow_unsafe_events.saturating_add(1);
            }
        }

        if frame > 0 && frame % LYAPUNOV_INTERVAL_FRAMES == 0 {
            let distance = voice.state.distance(shadow);
            if distance.is_finite() && distance > 0.0 {
                lyapunov_sum += (distance / LYAPUNOV_PERTURBATION).ln();
                lyapunov_count = lyapunov_count.saturating_add(1);
                let scale = LYAPUNOV_PERTURBATION / distance;
                shadow = voice.state.add_scaled(shadow.sub(voice.state), scale);
            } else {
                lyapunov_sum += -32.0;
                lyapunov_count = lyapunov_count.saturating_add(1);
                shadow = BcsState {
                    hopf_re: voice.state.hopf_re + LYAPUNOV_PERTURBATION,
                    ..voice.state
                };
            }
        }
    }

    let lyapunov_proxy = if lyapunov_count > 0 {
        lyapunov_sum / lyapunov_count as f32
    } else {
        0.0
    };
    let diagnostics = diagnostics_for_samples(
        &samples,
        &voice,
        gesture,
        sample_rate_hz,
        lyapunov_proxy,
        shadow_unsafe,
        shadow_unsafe_events,
    );

    if diagnostics.finite && !diagnostics.unsafe_state {
        Ok((samples, diagnostics))
    } else {
        Err(diagnostics)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BcsControl {
    drive: f32,
    edge: f32,
    duffing_mix: f32,
    output_gain: f32,
    hopf_mu_bias: f32,
    duffing_frequency_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BcsState {
    hopf_re: f32,
    hopf_im: f32,
    duffing_x: f32,
    duffing_v: f32,
}

impl BcsState {
    const fn seeded() -> Self {
        Self {
            hopf_re: 0.025,
            hopf_im: 0.0,
            duffing_x: 0.001,
            duffing_v: 0.0,
        }
    }

    fn is_finite(self) -> bool {
        self.hopf_re.is_finite()
            && self.hopf_im.is_finite()
            && self.duffing_x.is_finite()
            && self.duffing_v.is_finite()
    }

    fn max_abs(self) -> f32 {
        self.hopf_re
            .abs()
            .max(self.hopf_im.abs())
            .max(self.duffing_x.abs())
            .max(self.duffing_v.abs())
    }

    fn add_scaled(self, rhs: Self, scale: f32) -> Self {
        Self {
            hopf_re: self.hopf_re + rhs.hopf_re * scale,
            hopf_im: self.hopf_im + rhs.hopf_im * scale,
            duffing_x: self.duffing_x + rhs.duffing_x * scale,
            duffing_v: self.duffing_v + rhs.duffing_v * scale,
        }
    }

    fn sub(self, rhs: Self) -> Self {
        Self {
            hopf_re: self.hopf_re - rhs.hopf_re,
            hopf_im: self.hopf_im - rhs.hopf_im,
            duffing_x: self.duffing_x - rhs.duffing_x,
            duffing_v: self.duffing_v - rhs.duffing_v,
        }
    }

    fn distance(self, rhs: Self) -> f32 {
        let diff = self.sub(rhs);
        (diff.hopf_re * diff.hopf_re
            + diff.hopf_im * diff.hopf_im
            + diff.duffing_x * diff.duffing_x
            + diff.duffing_v * diff.duffing_v)
            .sqrt()
    }
}

fn rk4_step(state: BcsState, params: BcsParams, control: BcsControl, dt: f32) -> BcsState {
    let k1 = derivative(state, params, control);
    let k2 = derivative(state.add_scaled(k1, dt * 0.5), params, control);
    let k3 = derivative(state.add_scaled(k2, dt * 0.5), params, control);
    let k4 = derivative(state.add_scaled(k3, dt), params, control);

    state
        .add_scaled(k1, dt / 6.0)
        .add_scaled(k2, dt / 3.0)
        .add_scaled(k3, dt / 3.0)
        .add_scaled(k4, dt / 6.0)
}

fn derivative(state: BcsState, params: BcsParams, control: BcsControl) -> BcsState {
    let omega = TAU * params.base_frequency_hz;
    let hopf_r2 = state.hopf_re * state.hopf_re + state.hopf_im * state.hopf_im;
    let hopf_mu = params.hopf_mu + control.hopf_mu_bias;
    let duffing_feedback = state.duffing_x * params.duffing_feedback * omega;
    let hopf_radial = hopf_mu - params.hopf_nonlinearity * hopf_r2;

    let duffing_omega =
        omega * params.duffing_frequency_ratio * control.duffing_frequency_scale.max(0.05);
    let drive = params.duffing_drive * control.drive * state.hopf_re;
    let edge_damping =
        (params.duffing_edge * control.edge - params.duffing_damping) * duffing_omega;
    let nonlinear_damping =
        params.duffing_cubic_damping.abs() * duffing_omega * state.duffing_x * state.duffing_x;
    let cubic = params.duffing_cubic_stiffness * duffing_omega * state.duffing_x.powi(3);

    BcsState {
        hopf_re: hopf_radial * state.hopf_re - omega * state.hopf_im + duffing_feedback,
        hopf_im: omega * state.hopf_re + hopf_radial * state.hopf_im,
        duffing_x: duffing_omega * state.duffing_v,
        duffing_v: (edge_damping - nonlinear_damping) * state.duffing_v
            - duffing_omega * state.duffing_x
            - cubic
            + duffing_omega * drive,
    }
}

fn readout(state: BcsState, params: BcsParams, control: BcsControl) -> f32 {
    let hopf = state.hopf_re * params.hopf_mix;
    let duffing = state.duffing_x * params.duffing_mix * control.duffing_mix;
    soft_limit((hopf + duffing) * params.output_gain * control.output_gain)
}

fn diagnostics_for_samples(
    samples: &[f32],
    voice: &BcsVoice,
    gesture: BcsGesture,
    sample_rate_hz: u32,
    lyapunov_proxy: f32,
    shadow_unsafe: bool,
    shadow_unsafe_events: u64,
) -> BcsDiagnostics {
    let mut sum_squares = 0.0_f32;
    let mut peak_abs = 0.0_f32;
    let mut finite = true;

    for sample in samples {
        finite &= sample.is_finite();
        peak_abs = peak_abs.max(sample.abs());
        sum_squares += sample * sample;
    }

    let rms = if samples.is_empty() {
        0.0
    } else {
        (sum_squares / samples.len() as f32).sqrt()
    };
    let period = estimate_period_hz(
        samples,
        sample_rate_hz as f32,
        gesture.analysis_window(samples.len(), sample_rate_hz),
    );
    let subharmonic_ratio = period
        .estimated_period_hz
        .map(|period_hz| period_hz / BCS_V0_1_BASE_FREQUENCY_HZ);

    BcsDiagnostics {
        frames: samples.len(),
        rms,
        peak_abs,
        finite,
        max_state_abs: voice.max_state_abs(),
        unsafe_state: voice.unsafe_state() || shadow_unsafe,
        estimated_period_hz: period.estimated_period_hz,
        subharmonic_ratio,
        lyapunov_proxy,
        unsafe_events: voice.unsafe_events().saturating_add(shadow_unsafe_events),
        zero_crossings: period.zero_crossings,
        period_estimate_count: period.period_estimate_count,
    }
}

#[derive(Debug, Clone, Copy)]
struct PeriodEstimate {
    estimated_period_hz: Option<f32>,
    zero_crossings: usize,
    period_estimate_count: usize,
}

fn estimate_period_hz(
    samples: &[f32],
    sample_rate_hz: f32,
    window: Range<usize>,
) -> PeriodEstimate {
    let start = window.start.max(1).min(samples.len());
    let end = window.end.min(samples.len());
    let mut crossings = Vec::new();

    for index in start..end {
        let prev = samples[index - 1];
        let current = samples[index];
        if prev < 0.0 && current >= 0.0 {
            let denom = current - prev;
            let fraction = if denom.abs() > 0.000_000_1 {
                (-prev / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            crossings.push(index as f32 - 1.0 + fraction);
        }
    }

    let period_estimate_count = crossings.len().saturating_sub(1);
    let estimated_period_hz = if period_estimate_count >= 3 {
        let mut period_sum = 0.0_f32;
        for pair in crossings.windows(2) {
            period_sum += pair[1] - pair[0];
        }
        let average_period_frames = period_sum / period_estimate_count as f32;
        Some(sample_rate_hz / average_period_frames)
    } else {
        None
    };

    PeriodEstimate {
        estimated_period_hz,
        zero_crossings: crossings.len(),
        period_estimate_count,
    }
}

fn smoothstep(value: f32) -> f32 {
    let x = value.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn trapezoid(seconds: f32, start: f32, attack: f32, hold: f32, release: f32) -> f32 {
    let local = seconds - start;
    if local < 0.0 {
        0.0
    } else if local < attack {
        smoothstep(local / attack)
    } else if local < attack + hold {
        1.0
    } else if local < attack + hold + release {
        1.0 - smoothstep((local - attack - hold) / release)
    } else {
        0.0
    }
}

fn sanitize_positive(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn sanitize_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn soft_limit(sample: f32) -> f32 {
    (sample * 1.18).tanh().clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_scenario_params_rate_and_initial_state_render_same_buffer() {
        let params = BcsParams::for_scenario(BcsScenario::StableAnchor);
        let gesture = BcsGesture::new(BcsScenario::StableAnchor);
        let (first, _) = render_with_params(params, gesture).unwrap_or_else(|diagnostics| {
            panic!("first render failed: {diagnostics:?}");
        });
        let (second, _) = render_with_params(params, gesture).unwrap_or_else(|diagnostics| {
            panic!("second render failed: {diagnostics:?}");
        });

        assert_eq!(first.len(), second.len());
        assert!(
            first
                .iter()
                .zip(second.iter())
                .all(|(left, right)| left.to_bits() == right.to_bits())
        );
    }

    #[test]
    fn all_scenarios_produce_finite_bounded_output() {
        for scenario in BcsScenario::ALL {
            let (_, diagnostics) = render_scenario(scenario).unwrap_or_else(|diagnostics| {
                panic!("{scenario:?} render failed: {diagnostics:?}");
            });

            assert_eq!(
                diagnostics.frames,
                BCS_V0_1_SAMPLE_RATE_HZ as usize * BCS_V0_1_DURATION_SECONDS
            );
            assert!(diagnostics.finite);
            assert!(diagnostics.peak_abs <= 1.0);
            assert!(diagnostics.max_state_abs <= BCS_V0_1_STATE_CEILING);
            assert!(!diagnostics.unsafe_state);
            assert_eq!(diagnostics.unsafe_events, 0);
        }
    }

    #[test]
    fn stable_anchor_estimates_base_period() {
        let (_, diagnostics) =
            render_scenario(BcsScenario::StableAnchor).unwrap_or_else(|diagnostics| {
                panic!("stable render failed: {diagnostics:?}");
            });
        let ratio = diagnostics
            .subharmonic_ratio
            .unwrap_or_else(|| panic!("stable period estimate missing: {diagnostics:?}"));

        assert!((0.96..1.04).contains(&ratio), "ratio={ratio:?}");
        assert!(diagnostics.period_estimate_count > 300);
    }

    #[test]
    fn subharmonic_pressure_estimates_lower_period_ratio() {
        let (_, stable) =
            render_scenario(BcsScenario::StableAnchor).unwrap_or_else(|diagnostics| {
                panic!("stable render failed: {diagnostics:?}");
            });
        let (_, subharmonic) =
            render_scenario(BcsScenario::SubharmonicPressure).unwrap_or_else(|diagnostics| {
                panic!("subharmonic render failed: {diagnostics:?}");
            });
        let stable_ratio = stable
            .subharmonic_ratio
            .unwrap_or_else(|| panic!("stable period estimate missing: {stable:?}"));
        let subharmonic_ratio = subharmonic
            .subharmonic_ratio
            .unwrap_or_else(|| panic!("subharmonic period estimate missing: {subharmonic:?}"));

        assert!(subharmonic_ratio < stable_ratio * 0.70);
        assert!(
            (0.43..0.57).contains(&subharmonic_ratio),
            "ratio={subharmonic_ratio:?}"
        );
    }

    #[test]
    fn edge_sweep_has_higher_instability_proxy_than_stable_anchor() {
        let (_, stable) =
            render_scenario(BcsScenario::StableAnchor).unwrap_or_else(|diagnostics| {
                panic!("stable render failed: {diagnostics:?}");
            });
        let (_, edge) = render_scenario(BcsScenario::EdgeSweep).unwrap_or_else(|diagnostics| {
            panic!("edge render failed: {diagnostics:?}");
        });

        assert!(
            edge.lyapunov_proxy > stable.lyapunov_proxy
                || edge.max_state_abs > stable.max_state_abs * 1.10,
            "stable={stable:?} edge={edge:?}"
        );
    }

    #[test]
    fn recovery_return_ends_bounded_near_the_anchor_period() {
        let (_, diagnostics) =
            render_scenario(BcsScenario::RecoveryReturn).unwrap_or_else(|diagnostics| {
                panic!("recovery render failed: {diagnostics:?}");
            });
        let ratio = diagnostics
            .subharmonic_ratio
            .unwrap_or_else(|| panic!("recovery period estimate missing: {diagnostics:?}"));

        assert!(diagnostics.finite);
        assert!(!diagnostics.unsafe_state);
        assert!(diagnostics.peak_abs <= 1.0);
        assert!((0.94..1.06).contains(&ratio), "ratio={ratio:?}");
    }

    #[test]
    fn intentionally_extreme_params_fail_unsafe_detection() {
        let mut params = BcsParams::for_scenario(BcsScenario::StableAnchor);
        params.hopf_mu = 250.0;
        params.duffing_drive = 8.0;
        params.state_ceiling = 0.010;
        let result = render_with_params(params, BcsGesture::new(BcsScenario::StableAnchor));

        assert!(result.is_err());
        let diagnostics = result.err().unwrap_or_else(|| {
            panic!("unsafe render unexpectedly succeeded");
        });
        assert!(diagnostics.unsafe_state);
        assert!(diagnostics.unsafe_events > 0);
    }

    #[test]
    fn mamut_field_manifest_remains_dependency_free() {
        let manifest = include_str!("../Cargo.toml");

        assert!(manifest.contains("[dependencies]\n\n[lints]"));
        assert!(!manifest.contains("hound"));
    }
}
