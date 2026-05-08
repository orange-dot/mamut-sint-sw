use super::*;

impl Engine {
    pub fn gfm_layer_diagnostics(&self) -> Option<GfmDiagnostics> {
        self.gfm_layer_voice
            .as_ref()
            .map(GfmFieldVoice::diagnostics)
    }

    pub fn process_block(&mut self, block: ProcessBlock<'_>) {
        let note_events = block.note_events;
        let controller_events = block.controller_events;
        debug_assert!(is_sorted_by_frame(note_events));
        debug_assert!(is_sorted_by_frame(controller_events));

        let requested_frames = block.frame_count.min(self.config.max_block_frames);
        let mut output = block.output;
        let rendered_frames = output
            .as_ref()
            .map(|buffer| requested_frames.min(buffer.frames()))
            .unwrap_or(0);
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
        let mut output_safety = OutputSafetySnapshot::default();

        for frame in 0..requested_frames {
            while controller_index < controller_events.len()
                && controller_events[controller_index].frame_offset <= frame
            {
                self.handle_controller_event(controller_events[controller_index].event);
                controller_index += 1;
                events_processed += 1;
                self.refresh_resolved_state();
            }

            while note_index < note_events.len() && note_events[note_index].frame_offset <= frame {
                self.handle_note_event(note_events[note_index].event);
                note_index += 1;
                events_processed += 1;
            }

            let (left, right) = self.render_frame();
            let (left, right) = self.master_dc_blocker.process(left, right);
            output_safety.pre_safety_peak = output_safety
                .pre_safety_peak
                .max(left.abs().max(right.abs()));

            let limited_left = master_safety_limit(left);
            let limited_right = master_safety_limit(right);
            output_safety.observe_channel(left, limited_left);
            output_safety.observe_channel(right, limited_right);

            let left = sanitize_sample(limited_left);
            let right = sanitize_sample(limited_right);
            peak_output = peak_output.max(left.abs().max(right.abs()));
            output_safety.post_safety_peak = output_safety
                .post_safety_peak
                .max(left.abs().max(right.abs()));

            if frame < rendered_frames {
                let Some(buffer) = output.as_mut() else {
                    continue;
                };
                buffer.left[frame] = left;
                buffer.right[frame] = right;
            }
        }

        self.last_block_frames = requested_frames;
        self.last_events_processed = events_processed;
        self.last_peak_output = peak_output;
        self.last_output_safety = output_safety;
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
            patch_favorite: self
                .patch
                .ui
                .as_ref()
                .and_then(|ui| ui.favorite)
                .unwrap_or(false),
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
            performance_response: PerformanceResponseSnapshot {
                velocity_to_level: self.patch.performance_response.velocity_to_level,
                velocity_to_filter: self.patch.performance_response.velocity_to_filter,
                aftertouch_to_gravitacija: self
                    .patch
                    .performance_response
                    .aftertouch_to_gravitacija,
                aftertouch_to_baklja: self.patch.performance_response.aftertouch_to_baklja,
                mod_wheel_to_bloom: self.patch.performance_response.mod_wheel_to_bloom,
                mod_wheel_to_swarm: self.patch.performance_response.mod_wheel_to_swarm,
                bend_range_semitones: self.patch.performance_response.bend_range_semitones,
            },
            gfm_layer: self.gfm_layer_snapshot(),
            bcs_layer: self.bcs_layer_snapshot(),
            voices,
            held_notes,
            output_safety: self.last_output_safety,
            peak_output: self.last_peak_output,
            clip_detected: self.last_peak_output >= 0.98,
        }
    }
}
