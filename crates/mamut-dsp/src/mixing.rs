use crate::safety::soft_clip;

pub fn cross_mix_sample(mode_index: f32, osc1: f32, osc2: f32) -> f32 {
    match mode_index.round() as i32 {
        1 => osc1 * osc2,
        2 => soft_clip((osc1 + osc2) * 1.65, 0.0),
        3 => {
            if osc1.abs() >= osc2.abs() {
                osc1
            } else {
                osc2
            }
        }
        4 => (osc1 - osc2) * 0.70,
        _ => osc1 + osc2,
    }
}
