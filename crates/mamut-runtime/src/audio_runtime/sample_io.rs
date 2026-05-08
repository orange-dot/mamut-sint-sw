use super::*;

pub enum AlsaPlaybackWriter<'a> {
    Float32(IO<'a, f32>),
    Signed32 { io: IO<'a, i32>, buffer: Vec<i32> },
}

impl<'a> AlsaPlaybackWriter<'a> {
    pub(super) fn new(
        pcm: &'a PCM,
        sample_format: AlsaPlaybackSampleFormat,
        capacity_samples: usize,
    ) -> Result<Self> {
        match sample_format {
            AlsaPlaybackSampleFormat::Float32 => pcm
                .io_f32()
                .map(Self::Float32)
                .context("failed to create F32 ALSA IO"),
            AlsaPlaybackSampleFormat::Signed32 => Ok(Self::Signed32 {
                io: pcm.io_i32().context("failed to create S32 ALSA IO")?,
                buffer: vec![0; capacity_samples],
            }),
        }
    }

    pub(super) fn write_interleaved(
        &mut self,
        samples: &[f32],
    ) -> std::result::Result<usize, alsa::Error> {
        match self {
            Self::Float32(io) => io.writei(samples),
            Self::Signed32 { io, buffer } => {
                if buffer.len() < samples.len() {
                    buffer.resize(samples.len(), 0);
                }
                convert_f32_samples_to_s32(samples, &mut buffer[..samples.len()]);
                io.writei(&buffer[..samples.len()])
            }
        }
    }
}

pub fn convert_f32_samples_to_s32(samples: &[f32], output: &mut [i32]) {
    debug_assert!(output.len() >= samples.len());
    for (sample, target) in samples.iter().zip(output.iter_mut()) {
        *target = f32_sample_to_s32(*sample);
    }
}

pub fn f32_sample_to_s32(sample: f32) -> i32 {
    if !sample.is_finite() {
        return 0;
    }

    let sample = sample.clamp(-1.0, 1.0);
    if sample >= 1.0 {
        i32::MAX
    } else if sample <= -1.0 {
        i32::MIN
    } else if sample >= 0.0 {
        (sample * i32::MAX as f32).round() as i32
    } else {
        (sample * 2_147_483_648.0).round() as i32
    }
}

pub fn recover_alsa_playback_error(
    pcm: &PCM,
    transport_metrics: &TransportMetrics,
    error: alsa::Error,
    audio_selector: &str,
    audio_device_name: &str,
) -> Result<()> {
    match pcm.try_recover(error, true) {
        Ok(()) => {
            transport_metrics.record_xrun_recovery();
            Ok(())
        }
        Err(error) => {
            eprintln!(
                "audio stream error: ALSA playback failed on {} ({}): {error}",
                audio_selector, audio_device_name
            );
            Err(anyhow!(error))
        }
    }
}
