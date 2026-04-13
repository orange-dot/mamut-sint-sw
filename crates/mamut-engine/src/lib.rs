use mamut_dsp::{
    AdsrEnvelope, AdsrTiming, LinearSmoother, NoiseRng, Oscillator, SimpleChorus, SimpleReverb,
    StateVariableFilter, StereoBlockMut, db_to_gain, midi_note_hz, sanitize_sample, soft_clip,
};
use mamut_identity::{
    DerivedState, IdentityState, MacroState, ResolvedIdentityFrame, resolve_identity,
};
use mamut_params::{MacroId, ParamId, param_spec};
use mamut_patch::{PatchFileV1, PatchValidationError, validate_patch_v1};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    pub sample_rate_hz: f32,
    pub max_block_frames: usize,
    pub voice_count: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 48_000.0,
            max_block_frames: 256,
            voice_count: 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteEvent {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControllerEvent {
    PitchBend { semitones: f32 },
    ModWheel { amount: f32 },
    Sustain { down: bool },
    ChannelAftertouch { pressure: f32 },
    Macro { id: MacroId, value: f32 },
    DirectParam { id: ParamId, value: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scheduled<T> {
    pub frame_offset: usize,
    pub event: T,
}

pub type ScheduledNoteEvent = Scheduled<NoteEvent>;
pub type ScheduledControllerEvent = Scheduled<ControllerEvent>;

#[derive(Debug)]
pub struct ProcessBlock<'a> {
    pub frame_count: usize,
    pub note_events: &'a [ScheduledNoteEvent],
    pub controller_events: &'a [ScheduledControllerEvent],
    pub macro_state: Option<MacroState>,
    pub output: Option<StereoBlockMut<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicePhase {
    Idle,
    Held,
    Released,
    SustainedReleased,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceSnapshot {
    pub slot: usize,
    pub note: Option<u8>,
    pub velocity: f32,
    pub phase: VoicePhase,
    pub age: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectParameters {
    pub osc1_wave_mix: [f32; 4],
    pub osc2_wave_mix: [f32; 3],
    pub sub_level: f32,
    pub osc2_interval_semitones: f32,
    pub sync_amount: f32,
    pub crossmod_amount: f32,
    pub detune_spread_cents: f32,
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub filter_drive: f32,
    pub filter_env_depth: f32,
    pub filter_tracking: f32,
    pub amp_env: AdsrTiming,
    pub filter_env: AdsrTiming,
    pub voice_level: f32,
    pub body_drive: f32,
    pub output_trim_db: f32,
    pub stereo_width: f32,
    pub stereo_crossfeed: f32,
    pub final_saturation: f32,
    pub final_asymmetry: f32,
    pub low_mid_emphasis: f32,
    pub chorus_enabled: bool,
    pub chorus_mix: f32,
    pub chorus_depth: f32,
    pub chorus_rate_hz: f32,
    pub reverb_enabled: bool,
    pub reverb_mix: f32,
    pub reverb_size: f32,
    pub reverb_damping: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineSnapshot {
    pub patch_name: String,
    pub patch_description: Option<String>,
    pub patch_tags: Vec<String>,
    pub sample_rate_hz: f32,
    pub last_block_frames: usize,
    pub events_processed: usize,
    pub active_voice_count: usize,
    pub sustain_down: bool,
    pub live_macros: MacroState,
    pub effective_macros: MacroState,
    pub identity: IdentityState,
    pub derived: DerivedState,
    pub direct: DirectParameters,
    pub voices: Vec<VoiceSnapshot>,
    pub held_notes: Vec<u8>,
    pub peak_output: f32,
}

#[derive(Debug, Clone, Copy)]
struct ControlState {
    pitch_bend_semitones: f32,
    mod_wheel: f32,
    sustain_down: bool,
    aftertouch: f32,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            pitch_bend_semitones: 0.0,
            mod_wheel: 0.0,
            sustain_down: false,
            aftertouch: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct RenderSmoothers {
    sync_amount: LinearSmoother,
    crossmod_amount: LinearSmoother,
    cutoff_hz: LinearSmoother,
    resonance: LinearSmoother,
    filter_drive: LinearSmoother,
    voice_level: LinearSmoother,
    body_drive: LinearSmoother,
    stereo_width: LinearSmoother,
    stereo_crossfeed: LinearSmoother,
    final_saturation: LinearSmoother,
    final_asymmetry: LinearSmoother,
    low_mid_emphasis: LinearSmoother,
}

impl RenderSmoothers {
    fn new(direct: DirectParameters) -> Self {
        Self {
            sync_amount: LinearSmoother::new(direct.sync_amount),
            crossmod_amount: LinearSmoother::new(direct.crossmod_amount),
            cutoff_hz: LinearSmoother::new(direct.cutoff_hz),
            resonance: LinearSmoother::new(direct.resonance),
            filter_drive: LinearSmoother::new(direct.filter_drive),
            voice_level: LinearSmoother::new(direct.voice_level),
            body_drive: LinearSmoother::new(direct.body_drive),
            stereo_width: LinearSmoother::new(direct.stereo_width),
            stereo_crossfeed: LinearSmoother::new(direct.stereo_crossfeed),
            final_saturation: LinearSmoother::new(direct.final_saturation),
            final_asymmetry: LinearSmoother::new(direct.final_asymmetry),
            low_mid_emphasis: LinearSmoother::new(direct.low_mid_emphasis),
        }
    }

    fn set_targets(&mut self, direct: DirectParameters, sample_count: usize) {
        self.sync_amount
            .set_target(direct.sync_amount, sample_count);
        self.crossmod_amount
            .set_target(direct.crossmod_amount, sample_count);
        self.cutoff_hz.set_target(direct.cutoff_hz, sample_count);
        self.resonance.set_target(direct.resonance, sample_count);
        self.filter_drive
            .set_target(direct.filter_drive, sample_count);
        self.voice_level
            .set_target(direct.voice_level, sample_count);
        self.body_drive.set_target(direct.body_drive, sample_count);
        self.stereo_width
            .set_target(direct.stereo_width, sample_count);
        self.stereo_crossfeed
            .set_target(direct.stereo_crossfeed, sample_count);
        self.final_saturation
            .set_target(direct.final_saturation, sample_count);
        self.final_asymmetry
            .set_target(direct.final_asymmetry, sample_count);
        self.low_mid_emphasis
            .set_target(direct.low_mid_emphasis, sample_count);
    }

    fn next_direct(&mut self, mut direct: DirectParameters) -> DirectParameters {
        direct.sync_amount = self.sync_amount.next_value();
        direct.crossmod_amount = self.crossmod_amount.next_value();
        direct.cutoff_hz = self.cutoff_hz.next_value();
        direct.resonance = self.resonance.next_value();
        direct.filter_drive = self.filter_drive.next_value();
        direct.voice_level = self.voice_level.next_value();
        direct.body_drive = self.body_drive.next_value();
        direct.stereo_width = self.stereo_width.next_value();
        direct.stereo_crossfeed = self.stereo_crossfeed.next_value();
        direct.final_saturation = self.final_saturation.next_value();
        direct.final_asymmetry = self.final_asymmetry.next_value();
        direct.low_mid_emphasis = self.low_mid_emphasis.next_value();
        direct
    }
}

#[derive(Debug, Clone)]
struct VoiceState {
    note: Option<u8>,
    velocity: f32,
    phase: VoicePhase,
    age: u64,
    osc1: Oscillator,
    osc2: Oscillator,
    sub: Oscillator,
    noise: NoiseRng,
    filter: StateVariableFilter,
    amp_env: AdsrEnvelope,
    filter_env: AdsrEnvelope,
}

impl VoiceState {
    fn idle(sample_rate_hz: f32, slot: usize) -> Self {
        let seed = (0x1234_5678_u32).wrapping_add((slot as u32).wrapping_mul(0x9E37_79B9));
        Self {
            note: None,
            velocity: 0.0,
            phase: VoicePhase::Idle,
            age: 0,
            osc1: Oscillator::new(),
            osc2: Oscillator::new(),
            sub: Oscillator::new(),
            noise: NoiseRng::new(seed),
            filter: StateVariableFilter::new(),
            amp_env: AdsrEnvelope::new(
                sample_rate_hz,
                AdsrTiming {
                    attack_ms: 10.0,
                    decay_ms: 50.0,
                    sustain: 1.0,
                    release_ms: 100.0,
                },
            ),
            filter_env: AdsrEnvelope::new(
                sample_rate_hz,
                AdsrTiming {
                    attack_ms: 10.0,
                    decay_ms: 50.0,
                    sustain: 1.0,
                    release_ms: 100.0,
                },
            ),
        }
    }

    fn trigger(
        &mut self,
        note: u8,
        velocity: f32,
        age: u64,
        amp_env: AdsrTiming,
        filter_env: AdsrTiming,
        slot: usize,
    ) {
        self.note = Some(note);
        self.velocity = velocity.clamp(0.0, 1.0);
        self.phase = VoicePhase::Held;
        self.age = age;
        self.filter.reset();
        self.amp_env.set_timing(amp_env);
        self.filter_env.set_timing(filter_env);
        self.amp_env.note_on();
        self.filter_env.note_on();
        self.osc1
            .set_phase((((slot as f32) * 0.137) + note as f32 * 0.017).fract());
        self.osc2
            .set_phase((((slot as f32) * 0.223) + note as f32 * 0.031).fract());
        self.sub
            .set_phase((((slot as f32) * 0.089) + note as f32 * 0.013).fract());
    }

    fn start_release(&mut self) {
        if self.phase != VoicePhase::Idle {
            self.phase = VoicePhase::Released;
            self.amp_env.note_off();
            self.filter_env.note_off();
        }
    }
}

#[derive(Debug, Clone)]
pub struct Engine {
    config: EngineConfig,
    patch: PatchFileV1,
    live_macros: MacroState,
    control: ControlState,
    voices: Vec<VoiceState>,
    age_counter: u64,
    last_frame: ResolvedIdentityFrame,
    last_direct: DirectParameters,
    render_smoothers: RenderSmoothers,
    control_smoothing_samples: usize,
    last_block_frames: usize,
    last_events_processed: usize,
    last_peak_output: f32,
    final_body_left: f32,
    final_body_right: f32,
    chorus: SimpleChorus,
    reverb: SimpleReverb,
}

impl Engine {
    pub fn new(config: EngineConfig, patch: PatchFileV1) -> Result<Self, PatchValidationError> {
        validate_patch_v1(&patch)?;
        let config = EngineConfig {
            sample_rate_hz: config.sample_rate_hz.max(8_000.0),
            max_block_frames: config.max_block_frames.max(1),
            voice_count: config.voice_count.max(1),
        };
        let live_macros = MacroState::from_defaults(&patch.macros);
        let last_frame = resolve_identity(&patch, &live_macros);
        let last_direct = resolve_direct_parameters(&patch, last_frame, ControlState::default());
        let render_smoothers = RenderSmoothers::new(last_direct);
        let mut engine = Self {
            config,
            patch,
            live_macros,
            control: ControlState::default(),
            voices: (0..config.voice_count)
                .map(|slot| VoiceState::idle(config.sample_rate_hz, slot))
                .collect(),
            age_counter: 0,
            last_frame,
            last_direct,
            render_smoothers,
            control_smoothing_samples: control_smoothing_samples(config.sample_rate_hz),
            last_block_frames: 0,
            last_events_processed: 0,
            last_peak_output: 0.0,
            final_body_left: 0.0,
            final_body_right: 0.0,
            chorus: SimpleChorus::new(config.sample_rate_hz),
            reverb: SimpleReverb::new(config.sample_rate_hz),
        };
        engine.refresh_resolved_state();
        Ok(engine)
    }

    pub fn load_patch(&mut self, patch: PatchFileV1) -> Result<(), PatchValidationError> {
        validate_patch_v1(&patch)?;
        self.patch = patch;
        self.live_macros = MacroState::from_defaults(&self.patch.macros);
        self.reset_runtime_state();
        self.refresh_resolved_state();
        Ok(())
    }

    pub fn process_block(&mut self, block: ProcessBlock<'_>) {
        let frame_count = block.frame_count.min(self.config.max_block_frames);
        let mut output = block.output;
        if let Some(buffer) = output.as_mut() {
            buffer.clear();
        }

        if let Some(macro_state) = block.macro_state {
            self.live_macros = macro_state.clamped();
        }
        self.refresh_resolved_state();

        let mut note_index = 0;
        let mut controller_index = 0;
        let mut events_processed = 0;
        let mut peak_output: f32 = 0.0;

        for frame in 0..frame_count {
            while controller_index < block.controller_events.len()
                && block.controller_events[controller_index].frame_offset <= frame
            {
                self.handle_controller_event(block.controller_events[controller_index].event);
                controller_index += 1;
                events_processed += 1;
                self.refresh_resolved_state();
            }

            while note_index < block.note_events.len()
                && block.note_events[note_index].frame_offset <= frame
            {
                self.handle_note_event(block.note_events[note_index].event);
                note_index += 1;
                events_processed += 1;
            }

            let (left, right) = self.render_frame();
            peak_output = peak_output.max(left.abs().max(right.abs()));

            if let Some(buffer) = output.as_mut() {
                buffer.left[frame] = sanitize_sample(left);
                buffer.right[frame] = sanitize_sample(right);
            }
        }

        self.last_block_frames = frame_count;
        self.last_events_processed = events_processed;
        self.last_peak_output = peak_output;
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        let effective_macros = self.effective_macro_state();
        let voices: Vec<VoiceSnapshot> = self
            .voices
            .iter()
            .enumerate()
            .map(|(slot, voice)| VoiceSnapshot {
                slot,
                note: voice.note,
                velocity: voice.velocity,
                phase: voice.phase,
                age: voice.age,
            })
            .collect();

        let held_notes = voices
            .iter()
            .filter_map(|voice| match voice.phase {
                VoicePhase::Held | VoicePhase::Released | VoicePhase::SustainedReleased => {
                    voice.note
                }
                VoicePhase::Idle => None,
            })
            .collect();

        EngineSnapshot {
            patch_name: self.patch.meta.patch_name.clone(),
            patch_description: self.patch.meta.description.clone(),
            patch_tags: self.patch.meta.tags.clone().unwrap_or_default(),
            sample_rate_hz: self.config.sample_rate_hz,
            last_block_frames: self.last_block_frames,
            events_processed: self.last_events_processed,
            active_voice_count: self
                .voices
                .iter()
                .filter(|voice| voice.phase != VoicePhase::Idle)
                .count(),
            sustain_down: self.control.sustain_down,
            live_macros: self.live_macros,
            effective_macros,
            identity: self.last_frame.identity,
            derived: self.last_frame.derived,
            direct: self.last_direct,
            voices,
            held_notes,
            peak_output: self.last_peak_output,
        }
    }

    fn reset_runtime_state(&mut self) {
        self.control = ControlState::default();
        self.age_counter = 0;
        self.last_block_frames = 0;
        self.last_events_processed = 0;
        self.last_peak_output = 0.0;
        self.final_body_left = 0.0;
        self.final_body_right = 0.0;
        self.chorus = SimpleChorus::new(self.config.sample_rate_hz);
        self.reverb = SimpleReverb::new(self.config.sample_rate_hz);
        self.voices = (0..self.config.voice_count)
            .map(|slot| VoiceState::idle(self.config.sample_rate_hz, slot))
            .collect();
    }

    fn refresh_resolved_state(&mut self) {
        let effective_macros = self.effective_macro_state();
        self.last_frame = resolve_identity(&self.patch, &effective_macros);
        self.last_direct = resolve_direct_parameters(&self.patch, self.last_frame, self.control);
        self.render_smoothers
            .set_targets(self.last_direct, self.control_smoothing_samples);

        for voice in &mut self.voices {
            voice.amp_env.set_timing(self.last_direct.amp_env);
            voice.filter_env.set_timing(self.last_direct.filter_env);
        }
    }

    fn handle_note_event(&mut self, event: NoteEvent) {
        match event {
            NoteEvent::NoteOn { note, velocity } if velocity > 0.0 => self.note_on(note, velocity),
            NoteEvent::NoteOn { note, .. } | NoteEvent::NoteOff { note } => self.note_off(note),
        }
    }

    fn handle_controller_event(&mut self, event: ControllerEvent) {
        match event {
            ControllerEvent::PitchBend { semitones } => {
                self.control.pitch_bend_semitones = semitones;
            }
            ControllerEvent::ModWheel { amount } => {
                self.control.mod_wheel = amount.clamp(0.0, 1.0);
            }
            ControllerEvent::Sustain { down } => {
                self.control.sustain_down = down;
                if !down {
                    for voice in &mut self.voices {
                        if voice.phase == VoicePhase::SustainedReleased {
                            voice.start_release();
                        }
                    }
                }
            }
            ControllerEvent::ChannelAftertouch { pressure } => {
                self.control.aftertouch = pressure.clamp(0.0, 1.0);
            }
            ControllerEvent::Macro { id, value } => {
                self.live_macros.set(id, value.clamp(0.0, 1.0));
            }
            ControllerEvent::DirectParam { id, value } => self.apply_direct_param(id, value),
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let voice_index = self.allocate_voice_index();
        self.age_counter += 1;
        self.voices[voice_index].trigger(
            note,
            velocity,
            self.age_counter,
            self.last_direct.amp_env,
            self.last_direct.filter_env,
            voice_index,
        );
    }

    fn note_off(&mut self, note: u8) {
        if let Some(index) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.note == Some(note) && voice.phase != VoicePhase::Idle)
            .min_by_key(|(_, voice)| voice.age)
            .map(|(index, _)| index)
        {
            let voice = &mut self.voices[index];
            if self.control.sustain_down {
                voice.phase = VoicePhase::SustainedReleased;
            } else {
                voice.start_release();
            }
        }
    }

    fn allocate_voice_index(&self) -> usize {
        if let Some(index) = self
            .voices
            .iter()
            .position(|voice| voice.phase == VoicePhase::Idle)
        {
            return index;
        }

        if let Some((index, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.phase == VoicePhase::Released)
            .min_by_key(|(_, voice)| voice.age)
        {
            return index;
        }

        self.voices
            .iter()
            .enumerate()
            .min_by_key(|(_, voice)| voice.age)
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn effective_macro_state(&self) -> MacroState {
        let performance = self.patch.performance_response;
        let mut macros = self.live_macros;
        macros.gravitacija += self.control.aftertouch * performance.aftertouch_to_gravitacija;
        macros.ruin += self.control.aftertouch * performance.aftertouch_to_baklja;
        macros.bloom += self.control.mod_wheel * performance.mod_wheel_to_bloom;
        macros.swarm += self.control.mod_wheel * performance.mod_wheel_to_swarm;
        macros.clamped()
    }

    fn render_frame(&mut self) -> (f32, f32) {
        let direct = self.render_smoothers.next_direct(self.last_direct);
        let derived = self.last_frame.derived;
        let identity = self.last_frame.identity;
        let engine_patch = &self.patch.engine;
        let sample_rate_hz = self.config.sample_rate_hz;
        let pitch_bend_semitones = self.control.pitch_bend_semitones;
        let note_velocity_to_level = engine_patch
            .voice
            .velocity_to_level
            .unwrap_or(self.patch.performance_response.velocity_to_level)
            .clamp(0.0, 1.0);
        let note_velocity_to_filter = engine_patch
            .voice
            .velocity_to_filter
            .unwrap_or(self.patch.performance_response.velocity_to_filter)
            .clamp(0.0, 1.0);
        let voice_count = self.voices.len().max(1);
        let osc1_fine = engine_patch.osc1.fine_tune_cents.unwrap_or(0.0) / 100.0;
        let osc2_fine = engine_patch.osc2.fine_tune_cents / 100.0;
        let sub_octave = engine_patch.sub.octave_offset as f32 * 12.0;
        let mixer_body_gain = 0.4 + engine_patch.mixer.body_mix * 0.6;
        let pre_filter_gain = 1.0 + engine_patch.mixer.pre_filter_drive * 2.0;
        let voice_level_gain = 0.40 + direct.voice_level * 0.60;
        let stereo_width = direct.stereo_width;
        let strain_drive = 1.0 + derived.strain * 0.22;
        let strain_bias = identity.baklja_edge * 0.18;

        let mut left = 0.0;
        let mut right = 0.0;

        for (slot, voice) in self.voices.iter_mut().enumerate() {
            if voice.phase == VoicePhase::Idle {
                continue;
            }

            let Some(note) = voice.note else {
                continue;
            };

            let spread_position = if voice_count == 1 {
                0.0
            } else {
                (slot as f32 / (voice_count - 1) as f32) * 2.0 - 1.0
            };
            let spread_detune_semitones =
                spread_position * direct.detune_spread_cents * 0.5 / 100.0;
            let pulse_width = (0.50 + identity.baklja_edge * 0.18 - identity.horizont_air * 0.05)
                .clamp(0.08, 0.92);

            let osc1_note =
                note as f32 + pitch_bend_semitones + osc1_fine + spread_detune_semitones;
            let osc1_freq = midi_note_hz(osc1_note);
            let osc1_mix = mixed_wave(
                &voice.osc1,
                direct.osc1_wave_mix,
                pulse_width,
                &mut voice.noise,
            );
            let osc1_wrapped = voice.osc1.advance(osc1_freq, sample_rate_hz);

            if osc1_wrapped && direct.sync_amount > 0.0 {
                voice.osc2.hard_sync(direct.sync_amount);
            }

            let osc2_note =
                note as f32 + direct.osc2_interval_semitones + osc2_fine + spread_detune_semitones;
            let osc2_freq = midi_note_hz(osc2_note)
                * (1.0 + osc1_mix * direct.crossmod_amount * 0.25).clamp(0.25, 4.0);
            let osc2_mix = mixed_wave_osc2(&voice.osc2, direct.osc2_wave_mix, pulse_width);
            voice.osc2.advance(osc2_freq, sample_rate_hz);

            let sub_freq = midi_note_hz(note as f32 + pitch_bend_semitones + sub_octave);
            let sub_mix = voice.sub.square_sample() * direct.sub_level;
            voice.sub.advance(sub_freq, sample_rate_hz);

            let body_mix = sub_mix * mixer_body_gain * (0.62 + derived.mass * 0.46);
            let pre_filter = soft_clip(
                (osc1_mix + osc2_mix + body_mix) * (pre_filter_gain + direct.filter_drive * 0.8),
                strain_bias,
            );

            let filter_env = voice.filter_env.next_sample();
            let keytrack = (1.0 + ((note as f32 - 60.0) / 48.0) * direct.filter_tracking * 0.42)
                .clamp(0.55, 1.35);
            let velocity_filter = 1.0 + voice.velocity * note_velocity_to_filter * 0.85;
            let cutoff_hz = (direct.cutoff_hz
                * (0.42 + filter_env * direct.filter_env_depth * 0.78)
                * keytrack
                * velocity_filter)
                .clamp(20.0, sample_rate_hz * 0.42);
            let filtered = voice.filter.process(
                pre_filter,
                cutoff_hz,
                direct.resonance,
                direct.filter_drive,
                derived.strain,
                sample_rate_hz,
            );

            let amp = voice.amp_env.next_sample();
            let velocity_gain =
                1.0 - note_velocity_to_level + voice.velocity * note_velocity_to_level;
            let mut sample = filtered * amp * velocity_gain * voice_level_gain;
            sample = soft_clip(sample * strain_drive, direct.final_asymmetry * 0.14);

            if voice.phase != VoicePhase::Held && voice.amp_env.is_idle() {
                *voice = VoiceState::idle(sample_rate_hz, slot);
                continue;
            }

            let pan = spread_position * stereo_width;
            let left_gain = ((1.0 - pan) * 0.5).clamp(0.0, 1.0).sqrt();
            let right_gain = ((1.0 + pan) * 0.5).clamp(0.0, 1.0).sqrt();
            left += sample * left_gain;
            right += sample * right_gain;
        }

        self.final_body_left += (left - self.final_body_left) * (0.022 + derived.mass * 0.026);
        self.final_body_right += (right - self.final_body_right) * (0.022 + derived.mass * 0.026);

        let raw_mid = (left + right) * 0.5;
        let raw_side = (left - right) * 0.5;
        let body_mid = (self.final_body_left + self.final_body_right) * 0.5;
        let focus_amount = (derived.body_focus * 0.26
            + identity.grav_pull * 0.22
            + direct.stereo_crossfeed * 0.18)
            .clamp(0.0, 0.75);
        let saturated_mid = soft_clip(
            (raw_mid + body_mid * (0.30 + direct.low_mid_emphasis * 0.58))
                * (1.0 + direct.body_drive * 1.75 + direct.final_saturation * 1.25),
            direct.final_asymmetry * 0.70,
        );
        let saturated_side = soft_clip(
            raw_side * (1.0 + direct.final_saturation * 0.28),
            -direct.final_asymmetry * 0.18,
        ) * (1.0 - focus_amount);

        left = saturated_mid + saturated_side;
        right = saturated_mid - saturated_side;

        if direct.chorus_enabled {
            (left, right) = self.chorus.process(
                left,
                right,
                direct.chorus_mix,
                direct.chorus_depth,
                direct.chorus_rate_hz,
            );
        }

        if direct.reverb_enabled {
            (left, right) = self.reverb.process(
                left,
                right,
                direct.reverb_mix,
                direct.reverb_size,
                direct.reverb_damping,
            );
        }

        let crossfeed = (0.04 + direct.stereo_crossfeed * 0.16).clamp(0.0, 0.22);
        let crossfed_left = left * (1.0 - crossfeed) + right * crossfeed;
        let crossfed_right = right * (1.0 - crossfeed) + left * crossfeed;
        let output_gain = db_to_gain(direct.output_trim_db);

        (crossfed_left * output_gain, crossfed_right * output_gain)
    }

    fn apply_direct_param(&mut self, id: ParamId, value: f32) {
        let spec = param_spec(id);
        let clamped = value.clamp(spec.min, spec.max);

        match id {
            ParamId::GravitacijaMacro
            | ParamId::BloomMacro
            | ParamId::HeatMacro
            | ParamId::RuinMacro
            | ParamId::SwarmMacro => {
                let macro_id = spec.macro_id.unwrap_or(MacroId::Gravitacija);
                self.live_macros.set(macro_id, clamped);
                self.patch.macros =
                    macro_defaults_with_override(&self.patch.macros, macro_id, clamped);
            }
            ParamId::Osc1SawLevel => self.patch.engine.osc1.saw_level = clamped,
            ParamId::Osc1PulseLevel => self.patch.engine.osc1.pulse_level = clamped,
            ParamId::Osc1TriangleLevel => self.patch.engine.osc1.triangle_level = clamped,
            ParamId::Osc1NoiseLevel => self.patch.engine.osc1.noise_level = clamped,
            ParamId::Osc1FineTuneCents => self.patch.engine.osc1.fine_tune_cents = Some(clamped),
            ParamId::Osc2SawLevel => self.patch.engine.osc2.saw_level = clamped,
            ParamId::Osc2PulseLevel => self.patch.engine.osc2.pulse_level = clamped,
            ParamId::Osc2TriangleLevel => self.patch.engine.osc2.triangle_level = clamped,
            ParamId::Osc2IntervalSemitones => {
                self.patch.engine.osc2.interval_semitones = clamped.round() as i8
            }
            ParamId::Osc2FineTuneCents => self.patch.engine.osc2.fine_tune_cents = clamped,
            ParamId::Osc2SyncAmount => self.patch.engine.osc2.sync_amount = clamped,
            ParamId::Osc2CrossmodAmount => self.patch.engine.osc2.crossmod_amount = clamped,
            ParamId::SubLevel => self.patch.engine.sub.level = clamped,
            ParamId::SubOctaveOffset => self.patch.engine.sub.octave_offset = clamped.round() as i8,
            ParamId::MixerPreFilterDrive => self.patch.engine.mixer.pre_filter_drive = clamped,
            ParamId::MixerBodyMix => self.patch.engine.mixer.body_mix = clamped,
            ParamId::FilterCutoffHz => self.patch.engine.filter.cutoff_hz = clamped,
            ParamId::FilterResonance => self.patch.engine.filter.resonance = clamped,
            ParamId::FilterDrive => self.patch.engine.filter.drive = clamped,
            ParamId::FilterKeytrack => self.patch.engine.filter.keytrack = clamped,
            ParamId::AmpEnvAttackMs => self.patch.engine.amp_env.attack_ms = clamped,
            ParamId::AmpEnvDecayMs => self.patch.engine.amp_env.decay_ms = clamped,
            ParamId::AmpEnvSustain => self.patch.engine.amp_env.sustain = clamped,
            ParamId::AmpEnvReleaseMs => self.patch.engine.amp_env.release_ms = clamped,
            ParamId::FilterEnvAttackMs => self.patch.engine.filter_env.adsr.attack_ms = clamped,
            ParamId::FilterEnvDecayMs => self.patch.engine.filter_env.adsr.decay_ms = clamped,
            ParamId::FilterEnvSustain => self.patch.engine.filter_env.adsr.sustain = clamped,
            ParamId::FilterEnvReleaseMs => self.patch.engine.filter_env.adsr.release_ms = clamped,
            ParamId::FilterEnvDepth => self.patch.engine.filter_env.depth = clamped,
            ParamId::VoiceStereoWidth => self.patch.engine.voice.stereo_width = clamped,
            ParamId::VoiceDetuneSpreadCents => {
                self.patch.engine.voice.detune_spread_cents = clamped
            }
            ParamId::VoiceVelocityToLevel => {
                self.patch.engine.voice.velocity_to_level = Some(clamped)
            }
            ParamId::VoiceVelocityToFilter => {
                self.patch.engine.voice.velocity_to_filter = Some(clamped)
            }
            ParamId::FinalStageBodyDrive => self.patch.engine.final_stage.body_drive = clamped,
            ParamId::FinalStageAsymmetry => self.patch.engine.final_stage.asymmetry = clamped,
            ParamId::FinalStageLowMidEmphasis => {
                self.patch.engine.final_stage.low_mid_emphasis = clamped
            }
            ParamId::FinalStageOutputTrimDb => {
                self.patch.engine.final_stage.output_trim_db = clamped
            }
            ParamId::ChorusEnabled => self.patch.engine.fx.chorus.enabled = clamped >= 0.5,
            ParamId::ChorusMix => self.patch.engine.fx.chorus.mix = clamped,
            ParamId::ChorusDepth => self.patch.engine.fx.chorus.depth = clamped,
            ParamId::ChorusRateHz => self.patch.engine.fx.chorus.rate_hz = clamped,
            ParamId::ReverbEnabled => self.patch.engine.fx.reverb.enabled = clamped >= 0.5,
            ParamId::ReverbMix => self.patch.engine.fx.reverb.mix = clamped,
            ParamId::ReverbSize => self.patch.engine.fx.reverb.size = clamped,
            ParamId::ReverbDamping => self.patch.engine.fx.reverb.damping = clamped,
        }
    }
}

fn mixed_wave(
    oscillator: &Oscillator,
    mix_levels: [f32; 4],
    pulse_width: f32,
    noise: &mut NoiseRng,
) -> f32 {
    let saw = oscillator.saw_sample() * mix_levels[0];
    let pulse = oscillator.pulse_sample(pulse_width) * mix_levels[1];
    let triangle = oscillator.triangle_sample() * mix_levels[2];
    let noise = noise.next_bipolar() * mix_levels[3] * 0.85;
    normalize_weighted_mix(saw + pulse + triangle + noise, &mix_levels)
}

fn mixed_wave_osc2(oscillator: &Oscillator, mix_levels: [f32; 3], pulse_width: f32) -> f32 {
    let saw = oscillator.saw_sample() * mix_levels[0];
    let pulse = oscillator.pulse_sample(pulse_width) * mix_levels[1];
    let triangle = oscillator.triangle_sample() * mix_levels[2];
    normalize_weighted_mix(saw + pulse + triangle, &mix_levels)
}

fn normalize_weighted_mix<const N: usize>(sample_sum: f32, mix_levels: &[f32; N]) -> f32 {
    let normalizer = mix_levels.iter().copied().sum::<f32>().max(1.0);
    sample_sum / normalizer
}

fn control_smoothing_samples(sample_rate_hz: f32) -> usize {
    let smoothing_window = (sample_rate_hz * 0.006).round() as usize;
    smoothing_window.max(8).min(512)
}

fn resolve_direct_parameters(
    patch: &PatchFileV1,
    resolved_frame: ResolvedIdentityFrame,
    control: ControlState,
) -> DirectParameters {
    let engine = &patch.engine;
    let identity = resolved_frame.identity;
    let derived = resolved_frame.derived;
    let shaped = resolved_frame.shaped_macros;

    let cutoff_scale =
        0.90 + shaped.bloom * 0.62 + identity.horizont_air * 0.18 - shaped.gravitacija * 0.40;
    let cutoff_hz = (engine.filter.cutoff_hz * cutoff_scale).clamp(20.0, 20_000.0);
    let stereo_width = (engine.voice.stereo_width + identity.horizont_span * 0.18).clamp(0.0, 1.0);
    let stereo_crossfeed = (0.10 + derived.body_focus * 0.28 + identity.grav_pull * 0.10
        - derived.spatial_dispersion * 0.10)
        .clamp(0.0, 1.0);

    DirectParameters {
        osc1_wave_mix: [
            engine.osc1.saw_level,
            engine.osc1.pulse_level,
            engine.osc1.triangle_level,
            engine.osc1.noise_level,
        ],
        osc2_wave_mix: [
            engine.osc2.saw_level,
            engine.osc2.pulse_level,
            engine.osc2.triangle_level,
        ],
        sub_level: (engine.sub.level * (0.70 + derived.mass * 0.30)).clamp(0.0, 1.0),
        osc2_interval_semitones: engine.osc2.interval_semitones as f32
            + control.pitch_bend_semitones.clamp(
                -(patch.performance_response.bend_range_semitones as f32),
                patch.performance_response.bend_range_semitones as f32,
            ),
        sync_amount: (engine.osc2.sync_amount + identity.baklja_sync_bias * 0.30).clamp(0.0, 1.0),
        crossmod_amount: (engine.osc2.crossmod_amount + identity.baklja_ready * 0.24)
            .clamp(0.0, 1.0),
        detune_spread_cents: (engine.voice.detune_spread_cents * (0.72 + shaped.swarm * 0.42))
            .clamp(0.0, 50.0),
        cutoff_hz,
        resonance: (engine.filter.resonance + identity.baklja_edge * 0.14 + shaped.ruin * 0.06
            - derived.mass * 0.04)
            .clamp(0.0, 1.0),
        filter_drive: (engine.filter.drive + identity.pec_heat * 0.18 + derived.strain * 0.10)
            .clamp(0.0, 1.0),
        filter_env_depth: (engine.filter_env.depth + identity.horizont_open * 0.12).clamp(0.0, 1.0),
        filter_tracking: (engine.filter.keytrack * (0.92 - derived.mass * 0.12)).clamp(0.0, 1.0),
        amp_env: AdsrTiming {
            attack_ms: engine.amp_env.attack_ms,
            decay_ms: engine.amp_env.decay_ms,
            sustain: engine.amp_env.sustain,
            release_ms: engine.amp_env.release_ms,
        }
        .clamp(),
        filter_env: AdsrTiming {
            attack_ms: engine.filter_env.adsr.attack_ms,
            decay_ms: engine.filter_env.adsr.decay_ms,
            sustain: engine.filter_env.adsr.sustain,
            release_ms: engine.filter_env.adsr.release_ms,
        }
        .clamp(),
        voice_level: (0.75
            + derived.mass * 0.20
            + patch.performance_response.velocity_to_level * 0.05)
            .clamp(0.0, 1.0),
        body_drive: (engine.final_stage.body_drive + derived.body_focus * 0.25).clamp(0.0, 1.0),
        output_trim_db: engine.final_stage.output_trim_db,
        stereo_width,
        stereo_crossfeed,
        final_saturation: (engine.final_stage.body_drive
            + derived.mass * 0.14
            + derived.strain * 0.08)
            .clamp(0.0, 1.0),
        final_asymmetry: (engine.final_stage.asymmetry + identity.baklja_edge * 0.22)
            .clamp(0.0, 1.0),
        low_mid_emphasis: (engine.final_stage.low_mid_emphasis + derived.mass * 0.15)
            .clamp(0.0, 1.0),
        chorus_enabled: engine.fx.chorus.enabled,
        chorus_mix: engine.fx.chorus.mix,
        chorus_depth: engine.fx.chorus.depth,
        chorus_rate_hz: engine.fx.chorus.rate_hz,
        reverb_enabled: engine.fx.reverb.enabled,
        reverb_mix: engine.fx.reverb.mix,
        reverb_size: engine.fx.reverb.size,
        reverb_damping: engine.fx.reverb.damping,
    }
}

fn macro_defaults_with_override(
    defaults: &mamut_patch::MacroDefaults,
    macro_id: MacroId,
    value: f32,
) -> mamut_patch::MacroDefaults {
    let mut macros = MacroState::from_defaults(defaults);
    macros.set(macro_id, value);
    mamut_patch::MacroDefaults {
        gravitacija: macros.gravitacija,
        bloom: macros.bloom,
        heat: macros.heat,
        ruin: macros.ruin,
        swarm: macros.swarm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_patch::load_patch_toml;

    const MOLTEN_HORIZON: &str = include_str!("../../../patches/factory/molten-horizon.toml");

    fn fixture_engine() -> Engine {
        let patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        Engine::new(EngineConfig::default(), patch).expect("fixture must validate")
    }

    #[test]
    fn allocator_prefers_idle_then_released_then_oldest_active() {
        let mut engine = fixture_engine();

        for note in 60..66 {
            engine.process_block(ProcessBlock {
                frame_count: 64,
                note_events: &[Scheduled {
                    frame_offset: 0,
                    event: NoteEvent::NoteOn {
                        note,
                        velocity: 0.8,
                    },
                }],
                controller_events: &[],
                macro_state: None,
                output: None,
            });
        }

        let full_snapshot = engine.snapshot();
        assert_eq!(full_snapshot.active_voice_count, 6);
        assert_eq!(full_snapshot.voices[0].note, Some(60));

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[
                Scheduled {
                    frame_offset: 0,
                    event: NoteEvent::NoteOff { note: 60 },
                },
                Scheduled {
                    frame_offset: 1,
                    event: NoteEvent::NoteOn {
                        note: 72,
                        velocity: 0.8,
                    },
                },
            ],
            controller_events: &[],
            macro_state: None,
            output: None,
        });

        let released_reused = engine.snapshot();
        assert!(released_reused.held_notes.contains(&72));

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 73,
                    velocity: 0.8,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: None,
        });

        let stolen = engine.snapshot();
        assert!(stolen.held_notes.contains(&73));
        assert!(!stolen.held_notes.contains(&61));
    }

    #[test]
    fn sustain_state_transitions_are_stable() {
        let mut engine = fixture_engine();

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 1.0,
                },
            }],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Sustain { down: true },
            }],
            macro_state: None,
            output: None,
        });

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOff { note: 60 },
            }],
            controller_events: &[],
            macro_state: None,
            output: None,
        });

        let sustained = engine.snapshot();
        assert_eq!(sustained.voices[0].phase, VoicePhase::SustainedReleased);

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Sustain { down: false },
            }],
            macro_state: None,
            output: None,
        });
        assert_eq!(engine.snapshot().voices[0].phase, VoicePhase::Released);
    }

    #[test]
    fn rendered_audio_is_non_silent_and_finite() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 256];
        let mut right = [0.0_f32; 256];
        engine.process_block(ProcessBlock {
            frame_count: 256,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.9,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        let peak = left
            .iter()
            .zip(right.iter())
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0, f32::max);
        assert!(peak > 0.0001);
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn macro_sweeps_remain_finite() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 512];
        let mut right = [0.0_f32; 512];

        engine.process_block(ProcessBlock {
            frame_count: 512,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 48,
                    velocity: 0.92,
                },
            }],
            controller_events: &[
                Scheduled {
                    frame_offset: 64,
                    event: ControllerEvent::Macro {
                        id: MacroId::Gravitacija,
                        value: 0.84,
                    },
                },
                Scheduled {
                    frame_offset: 160,
                    event: ControllerEvent::Macro {
                        id: MacroId::Ruin,
                        value: 0.76,
                    },
                },
                Scheduled {
                    frame_offset: 320,
                    event: ControllerEvent::Macro {
                        id: MacroId::Bloom,
                        value: 0.72,
                    },
                },
            ],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
        assert!(
            left.iter()
                .zip(right.iter())
                .map(|(left, right)| left.abs().max(right.abs()))
                .fold(0.0, f32::max)
                > 0.0001
        );
    }

    #[test]
    fn dry_path_remains_non_silent_with_fx_disabled() {
        let mut patch = load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse");
        patch.engine.fx.chorus.enabled = false;
        patch.engine.fx.reverb.enabled = false;
        let mut engine =
            Engine::new(EngineConfig::default(), patch).expect("fixture must validate");
        let mut left = [0.0_f32; 512];
        let mut right = [0.0_f32; 512];

        engine.process_block(ProcessBlock {
            frame_count: 512,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 43,
                    velocity: 0.88,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        let peak = left
            .iter()
            .zip(right.iter())
            .map(|(left, right)| left.abs().max(right.abs()))
            .fold(0.0, f32::max);
        assert!(peak > 0.0001);
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn dense_chord_playback_with_fx_stays_finite() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 1024];
        let mut right = [0.0_f32; 1024];
        let note_events = [
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 48,
                    velocity: 0.82,
                },
            },
            Scheduled {
                frame_offset: 32,
                event: NoteEvent::NoteOn {
                    note: 55,
                    velocity: 0.84,
                },
            },
            Scheduled {
                frame_offset: 64,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.88,
                },
            },
            Scheduled {
                frame_offset: 96,
                event: NoteEvent::NoteOn {
                    note: 67,
                    velocity: 0.90,
                },
            },
        ];
        let controller_events = [
            Scheduled {
                frame_offset: 128,
                event: ControllerEvent::Macro {
                    id: MacroId::Gravitacija,
                    value: 0.82,
                },
            },
            Scheduled {
                frame_offset: 256,
                event: ControllerEvent::Macro {
                    id: MacroId::Heat,
                    value: 0.74,
                },
            },
            Scheduled {
                frame_offset: 384,
                event: ControllerEvent::Macro {
                    id: MacroId::Ruin,
                    value: 0.70,
                },
            },
        ];

        engine.process_block(ProcessBlock {
            frame_count: 1024,
            note_events: &note_events,
            controller_events: &controller_events,
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });

        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(right.iter().all(|sample| sample.is_finite()));
        assert!(
            left.iter()
                .zip(right.iter())
                .map(|(left, right)| left.abs().max(right.abs()))
                .fold(0.0, f32::max)
                > 0.0001
        );
    }

    #[test]
    fn snapshot_exposes_patch_metadata() {
        let engine = fixture_engine();
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.patch_name, "Molten Horizon");
        assert!(snapshot.patch_description.is_some());
        assert!(snapshot.patch_tags.iter().any(|tag| tag == "monster"));
    }

    #[test]
    fn load_patch_resets_runtime_state() {
        let mut engine = fixture_engine();
        let mut left = [0.0_f32; 128];
        let mut right = [0.0_f32; 128];

        engine.process_block(ProcessBlock {
            frame_count: 128,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.9,
                },
            }],
            controller_events: &[Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Macro {
                    id: MacroId::Gravitacija,
                    value: 0.92,
                },
            }],
            macro_state: None,
            output: Some(StereoBlockMut::new(&mut left, &mut right)),
        });
        assert!(engine.snapshot().active_voice_count > 0);

        let replacement = load_patch_toml(include_str!("../../../patches/factory/ember-vault.toml"))
            .expect("replacement patch parses");
        engine.load_patch(replacement).expect("replacement patch loads");

        let snapshot = engine.snapshot();
        assert_eq!(snapshot.patch_name, "Ember Vault");
        assert_eq!(snapshot.active_voice_count, 0);
        assert!(!snapshot.sustain_down);
        assert!(snapshot.peak_output.abs() <= f32::EPSILON);
    }
}
