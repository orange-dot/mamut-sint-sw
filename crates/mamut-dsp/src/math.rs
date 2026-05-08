pub fn lerp(a: f32, b: f32, amount: f32) -> f32 {
    a + (b - a) * amount
}

pub fn mix(a: f32, b: f32, amount: f32) -> f32 {
    lerp(a, b, amount.clamp(0.0, 1.0))
}

pub fn unipolar_to_bipolar(value: f32) -> f32 {
    value.clamp(0.0, 1.0) * 2.0 - 1.0
}

pub fn bipolar_to_unipolar(value: f32) -> f32 {
    (value.clamp(-1.0, 1.0) + 1.0) * 0.5
}

pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

pub fn gain_to_db(gain: f32) -> f32 {
    let gain = if gain.is_finite() {
        gain.max(f32::MIN_POSITIVE)
    } else if gain.is_sign_positive() {
        f32::MAX
    } else {
        f32::MIN_POSITIVE
    };
    20.0 * gain.log10()
}

pub fn semitones_to_ratio(semitones: f32) -> f32 {
    2.0_f32.powf(semitones / 12.0)
}

pub fn cents_to_ratio(cents: f32) -> f32 {
    2.0_f32.powf(cents / 1200.0)
}

pub fn midi_note_to_hz(note: f32) -> f32 {
    440.0 * semitones_to_ratio(note - 69.0)
}

pub fn midi_note_hz(note: f32) -> f32 {
    midi_note_to_hz(note)
}

pub fn equal_power_pan(pan: f32) -> (f32, f32) {
    let pan = if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let left = ((1.0 - pan) * 0.5).sqrt();
    let right = ((1.0 + pan) * 0.5).sqrt();
    (left, right)
}

pub fn soft_clip(sample: f32, asymmetry: f32) -> f32 {
    let offset = asymmetry.clamp(-1.0, 1.0) * 0.35;
    ((sample + offset) * 1.25).tanh()
}

pub fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}
