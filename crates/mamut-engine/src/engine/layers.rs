use super::*;

impl Engine {
    pub(super) fn apply_gfm_layer(
        &mut self,
        left: f32,
        right: f32,
        direct: DirectParameters,
    ) -> (f32, f32) {
        let Some(program_id) = self.gfm_layer_voice.as_ref().map(GfmFieldVoice::program_id) else {
            return (left, right);
        };

        let pressure = self.gfm_layer_pressure.next_value();
        let effective_amount = self.gfm_layer_mix.next_value();
        let gfm_sample = {
            let Some(voice) = self.gfm_layer_voice.as_mut() else {
                return (left, right);
            };
            voice.next_sample_with_live_control(pressure, effective_amount)
        };
        if effective_amount <= f32::EPSILON {
            self.finish_pending_auto_disarm_if_silent();
            return (left, right);
        }
        let activity = ((left.abs() + right.abs()) * 0.75).clamp(0.0, 1.0);
        let gate = if activity <= 0.000_01 {
            0.0
        } else {
            (0.20 + activity * 0.80).clamp(0.0, 1.0)
        };
        let layer = gfm_sample * gate * gfm_engine_layer_gain(program_id) * effective_amount;
        let spread = (direct.stereo_width * 0.16 + direct.stereo_crossfeed * 0.04).clamp(0.0, 0.22);
        let left = soft_clip(left + layer * (1.0 - spread), direct.final_asymmetry * 0.20);
        let right = soft_clip(
            right + layer * (1.0 + spread),
            -direct.final_asymmetry * 0.20,
        );

        (left, right)
    }

    pub(super) fn apply_bcs_layer(
        &mut self,
        left: f32,
        right: f32,
        direct: DirectParameters,
    ) -> (f32, f32) {
        let BcsLayerMode::Enabled { scenario } = self.bcs_layer_mode else {
            return (left, right);
        };

        let effective_amount = self.bcs_layer_mix.next_value();
        let Some(voice) = self.bcs_layer_voice.as_mut() else {
            return (left, right);
        };

        let bcs_sample = voice.next_sample(BcsGesture::new(scenario));
        if voice.unsafe_state() || !bcs_sample.is_finite() {
            return (left, right);
        }
        if effective_amount <= f32::EPSILON {
            return (left, right);
        }

        let activity = ((left.abs() + right.abs()) * 0.75).clamp(0.0, 1.0);
        if activity <= 0.000_01 {
            return (left, right);
        }

        let gate = (0.18 + activity * 0.82).clamp(0.0, 1.0);
        let layer_gain = BCS_ENGINE_LAYER_GAIN * effective_amount;
        let layer = bcs_sample * layer_gain * gate;
        let spread = (direct.stereo_width * 0.10 + direct.stereo_crossfeed * 0.03).clamp(0.0, 0.16);
        let left = soft_clip(left + layer * (1.0 - spread), direct.final_asymmetry * 0.16);
        let right = soft_clip(
            right + layer * (1.0 + spread),
            -direct.final_asymmetry * 0.16,
        );

        (left, right)
    }
}
