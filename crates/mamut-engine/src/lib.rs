use mamut_dsp::{AdsrTiming, StereoBlockMut, sanitize_block};
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

#[derive(Debug, Clone, Copy)]
struct VoiceState {
    note: Option<u8>,
    velocity: f32,
    phase: VoicePhase,
    age: u64,
    release_blocks: u32,
}

impl VoiceState {
    fn idle() -> Self {
        Self {
            note: None,
            velocity: 0.0,
            phase: VoicePhase::Idle,
            age: 0,
            release_blocks: 0,
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
    last_block_frames: usize,
    last_events_processed: usize,
    last_peak_output: f32,
}

impl Engine {
    pub fn new(config: EngineConfig, patch: PatchFileV1) -> Result<Self, PatchValidationError> {
        validate_patch_v1(&patch)?;
        let config = EngineConfig {
            voice_count: config.voice_count.max(1),
            ..config
        };
        let live_macros = MacroState::from_defaults(&patch.macros);
        let last_frame = resolve_identity(&patch, &live_macros);
        let last_direct = resolve_direct_parameters(&patch, last_frame, ControlState::default());

        Ok(Self {
            config,
            patch,
            live_macros,
            control: ControlState::default(),
            voices: vec![VoiceState::idle(); config.voice_count.max(1)],
            age_counter: 0,
            last_frame,
            last_direct,
            last_block_frames: 0,
            last_events_processed: 0,
            last_peak_output: 0.0,
        })
    }

    pub fn load_patch(&mut self, patch: PatchFileV1) -> Result<(), PatchValidationError> {
        validate_patch_v1(&patch)?;
        self.patch = patch;
        self.live_macros = MacroState::from_defaults(&self.patch.macros);
        self.last_frame = resolve_identity(&self.patch, &self.live_macros);
        self.last_direct = resolve_direct_parameters(&self.patch, self.last_frame, self.control);
        Ok(())
    }

    pub fn process_block(&mut self, mut block: ProcessBlock<'_>) {
        if let Some(output) = block.output.as_mut() {
            output.clear();
        }

        self.process_events(block.note_events, block.controller_events);
        if let Some(macro_state) = block.macro_state {
            self.live_macros = macro_state.clamped();
        }

        let effective_macros = self.effective_macro_state();
        self.last_frame = resolve_identity(&self.patch, &effective_macros);
        self.last_direct = resolve_direct_parameters(&self.patch, self.last_frame, self.control);
        self.advance_release_state();

        if let Some(output) = block.output.as_mut() {
            sanitize_block(output);
            self.last_peak_output = output.peak_abs();
        } else {
            self.last_peak_output = 0.0;
        }

        self.last_block_frames = block.frame_count;
        self.last_events_processed = block.note_events.len() + block.controller_events.len();
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

    fn process_events(
        &mut self,
        note_events: &[ScheduledNoteEvent],
        controller_events: &[ScheduledControllerEvent],
    ) {
        let mut note_index = 0;
        let mut controller_index = 0;

        while note_index < note_events.len() || controller_index < controller_events.len() {
            let next_note = note_events.get(note_index);
            let next_controller = controller_events.get(controller_index);

            match (next_note, next_controller) {
                (Some(note_event), Some(controller_event))
                    if controller_event.frame_offset <= note_event.frame_offset =>
                {
                    self.handle_controller_event(controller_event.event);
                    controller_index += 1;
                }
                (Some(note_event), _) => {
                    self.handle_note_event(note_event.event);
                    note_index += 1;
                }
                (None, Some(controller_event)) => {
                    self.handle_controller_event(controller_event.event);
                    controller_index += 1;
                }
                (None, None) => break,
            }
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
                            voice.phase = VoicePhase::Released;
                            voice.release_blocks = 0;
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
            ControllerEvent::DirectParam { id, value } => {
                self.apply_direct_param(id, value);
            }
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let voice_index = self.allocate_voice_index();
        self.age_counter += 1;
        self.voices[voice_index] = VoiceState {
            note: Some(note),
            velocity: velocity.clamp(0.0, 1.0),
            phase: VoicePhase::Held,
            age: self.age_counter,
            release_blocks: 0,
        };
    }

    fn note_off(&mut self, note: u8) {
        let candidate = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.note == Some(note) && voice.phase != VoicePhase::Idle)
            .min_by_key(|(_, voice)| voice.age)
            .map(|(index, _)| index);

        if let Some(index) = candidate {
            let voice = &mut self.voices[index];
            if self.control.sustain_down {
                voice.phase = VoicePhase::SustainedReleased;
            } else {
                voice.phase = VoicePhase::Released;
                voice.release_blocks = 0;
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

    fn advance_release_state(&mut self) {
        for voice in &mut self.voices {
            if voice.phase == VoicePhase::Released {
                if voice.release_blocks >= 1 {
                    *voice = VoiceState::idle();
                } else {
                    voice.release_blocks += 1;
                }
            }
        }
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

fn resolve_direct_parameters(
    patch: &PatchFileV1,
    resolved_frame: ResolvedIdentityFrame,
    control: ControlState,
) -> DirectParameters {
    let engine = &patch.engine;
    let identity = resolved_frame.identity;
    let derived = resolved_frame.derived;

    let cutoff_scale = 1.0 + resolved_frame.shaped_macros.bloom * 0.55
        - resolved_frame.shaped_macros.gravitacija * 0.35;
    let cutoff_hz = (engine.filter.cutoff_hz * cutoff_scale).clamp(20.0, 20_000.0);
    let stereo_width = (engine.voice.stereo_width + identity.horizont_span * 0.18).clamp(0.0, 1.0);
    let stereo_crossfeed = (1.0 - derived.spatial_dispersion * 0.75).clamp(0.0, 1.0);

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
        detune_spread_cents: (engine.voice.detune_spread_cents
            * (0.75 + resolved_frame.shaped_macros.swarm * 0.45))
            .clamp(0.0, 50.0),
        cutoff_hz,
        resonance: (engine.filter.resonance + identity.baklja_edge * 0.18).clamp(0.0, 1.0),
        filter_drive: (engine.filter.drive + identity.pec_heat * 0.25).clamp(0.0, 1.0),
        filter_env_depth: (engine.filter_env.depth + identity.horizont_open * 0.12).clamp(0.0, 1.0),
        filter_tracking: engine.filter.keytrack,
        amp_env: AdsrTiming {
            attack_ms: engine.amp_env.attack_ms,
            decay_ms: engine.amp_env.decay_ms,
            sustain: engine.amp_env.sustain,
            release_ms: engine.amp_env.release_ms,
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
        final_saturation: (engine.final_stage.body_drive + derived.mass * 0.18).clamp(0.0, 1.0),
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

        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[],
            controller_events: &[],
            macro_state: None,
            output: None,
        });
        assert_eq!(engine.snapshot().voices[0].phase, VoicePhase::Idle);
    }
}
