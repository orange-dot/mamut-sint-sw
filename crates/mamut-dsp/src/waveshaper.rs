pub fn tanh_drive(sample: f32, drive: f32) -> f32 {
    let sample = finite_or(sample, 0.0);
    let drive = finite_or(drive, 0.0).clamp(0.0, 1.0);
    let gain = 1.0 + drive * 7.0;
    (sample * gain).tanh()
}

pub fn tanh_drive_compensated(sample: f32, drive: f32) -> f32 {
    let sample = finite_or(sample, 0.0);
    let drive = finite_or(drive, 0.0).clamp(0.0, 1.0);
    if drive <= f32::EPSILON {
        return sample.clamp(-1.0, 1.0);
    }
    let gain = 1.0 + drive * 7.0;
    let norm = gain.tanh().max(f32::MIN_POSITIVE);
    ((sample * gain).tanh() / norm).clamp(-1.0, 1.0)
}

pub fn cubic_soft_clip(sample: f32) -> f32 {
    let sample = finite_or(sample, 0.0).clamp(-1.5, 1.5);
    if sample.abs() <= 1.0 {
        sample - sample * sample * sample / 3.0
    } else {
        sample.signum() * 2.0 / 3.0
    }
}

pub fn cubic_soft_clip_compensated(sample: f32) -> f32 {
    (cubic_soft_clip(sample) * 1.5).clamp(-1.0, 1.0)
}

pub fn asymmetric_diode(sample: f32, drive: f32, asymmetry: f32) -> f32 {
    let sample = finite_or(sample, 0.0);
    let drive = finite_or(drive, 0.0).clamp(0.0, 1.0);
    let asymmetry = finite_or(asymmetry, 0.0).clamp(-1.0, 1.0);
    let positive_gain = 1.0 + drive * (3.0 + asymmetry.max(0.0) * 2.0);
    let negative_gain = 1.0 + drive * (3.0 + (-asymmetry).max(0.0) * 2.0);
    let bias = asymmetry * drive * 0.16;
    let shaped = if sample >= 0.0 {
        (sample * positive_gain + bias).tanh()
    } else {
        (sample * negative_gain + bias).tanh()
    };
    (shaped - bias * 0.35).clamp(-1.0, 1.0)
}

pub fn foldback(sample: f32, threshold: f32) -> f32 {
    let sample = finite_or(sample, 0.0);
    let threshold = finite_or(threshold, 1.0).clamp(0.05, 1.0);
    if sample.abs() <= threshold {
        return sample;
    }
    let period = threshold * 4.0;
    let folded = (sample + threshold).rem_euclid(period);
    let folded = if folded > threshold * 2.0 {
        period - folded
    } else {
        folded
    };
    (folded - threshold).clamp(-threshold, threshold) / threshold
}

pub fn drive_gain_compensated(sample: f32, drive: f32) -> f32 {
    let sample = finite_or(sample, 0.0);
    let drive = finite_or(drive, 0.0).clamp(0.0, 1.0);
    sample * (1.0 / (1.0 + drive * 1.75))
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
