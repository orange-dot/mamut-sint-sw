use super::*;

pub fn run_alsa_playback_loop(
    opened: OpenedAlsaPlayback,
    mut consumer: Consumer<StereoFrame>,
    transport_metrics: Arc<TransportMetrics>,
    stop: Arc<AtomicBool>,
) {
    let OpenedAlsaPlayback {
        pcm,
        audio_selector,
        audio_device_name,
        sample_rate_hz: _sample_rate_hz,
        channels,
        sample_format,
        tuning,
    } = opened;

    let mut writer = match AlsaPlaybackWriter::new(
        &pcm,
        sample_format,
        tuning.period_frames * channels.max(1),
    ) {
        Ok(writer) => writer,
        Err(error) => {
            eprintln!(
                "audio stream error: failed to create ALSA io for {}: {error}",
                audio_selector
            );
            return;
        }
    };
    let channel_count = channels.max(1);
    let mut output = vec![0.0_f32; tuning.period_frames * channel_count];

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        match pcm.wait(Some(ALSA_WAIT_TIMEOUT_MS)) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) => {
                if recover_alsa_playback_error(
                    &pcm,
                    &transport_metrics,
                    error,
                    &audio_selector,
                    &audio_device_name,
                )
                .is_ok()
                {
                    continue;
                }
                break;
            }
        }

        let available_frames = match pcm.avail_update() {
            Ok(frames) if frames > 0 => frames as usize,
            Ok(_) => continue,
            Err(error) => {
                if recover_alsa_playback_error(
                    &pcm,
                    &transport_metrics,
                    error,
                    &audio_selector,
                    &audio_device_name,
                )
                .is_ok()
                {
                    continue;
                }
                break;
            }
        };

        let frames_to_write = available_frames.min(tuning.period_frames).max(1);
        let sample_count = frames_to_write * channel_count;
        drain_queue_into_output(
            &mut consumer,
            &transport_metrics,
            &mut output[..sample_count],
            channel_count,
        );

        let mut written_frames = 0_usize;
        while written_frames < frames_to_write {
            let start = written_frames * channel_count;
            let end = frames_to_write * channel_count;
            match writer.write_interleaved(&output[start..end]) {
                Ok(0) => break,
                Ok(written_now) => written_frames += written_now,
                Err(error) => {
                    if recover_alsa_playback_error(
                        &pcm,
                        &transport_metrics,
                        error,
                        &audio_selector,
                        &audio_device_name,
                    )
                    .is_ok()
                    {
                        break;
                    }
                    return;
                }
            }
        }
    }
}
