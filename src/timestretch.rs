//! Time-stretching via rubato 1.x asynchronous resampler (sinc interpolation).

use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{
    Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

/// Applies time-stretching acceleration to the input audio.
pub fn apply_time_stretch(
    input_samples: &[i16],
    channels: usize,
    input_sample_rate: u32,
    config: &crate::model::TimeStretchConfig,
) -> Vec<i16> {
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        oversampling_factor: 256,
        interpolation: SincInterpolationType::Linear,
        window: WindowFunction::BlackmanHarris2,
    };

    let base_ratio = 1.0;
    let max_relative_ratio = config.target_speed * 1.1;
    let chunk_size = (input_sample_rate as usize / 30).max(4096);

    let mut resampler = Async::<f32>::new_sinc(
        base_ratio,
        max_relative_ratio,
        &params,
        chunk_size,
        channels,
        FixedAsync::Input,
    )
    .expect("failed to create resampler");

    let min_allowed_ratio = base_ratio / max_relative_ratio;
    let max_allowed_ratio = base_ratio * max_relative_ratio;

    let mut input_frames: Vec<Vec<f32>> = (0..channels).map(|_| Vec::new()).collect();
    let mut output_audio: Vec<i16> = Vec::new();

    for (i, &sample) in input_samples.iter().enumerate() {
        let sample_f32 = f32::from(sample) / f32::from(i16::MAX);
        input_frames[i % channels].push(sample_f32);
    }

    let output_chunk_capacity = resampler.output_frames_max();
    let mut output_chunk: Vec<Vec<f32>> = (0..channels)
        .map(|_| vec![0.0; output_chunk_capacity])
        .collect();

    let mut processed_frames: u64 = 0;
    let mut indexing = Indexing {
        input_offset: 0,
        output_offset: 0,
        partial_len: None,
        active_channels_mask: None,
    };

    while input_frames[0].len() >= resampler.input_frames_next() {
        #[allow(clippy::cast_precision_loss)]
        let processed_seconds = processed_frames as f64 / f64::from(input_sample_rate);

        let progress = if processed_seconds < config.initial_delay {
            0.0
        } else {
            ((processed_seconds - config.initial_delay) / config.ramp_duration).min(1.0)
        };

        let current_speed = 1.0 + (config.target_speed - 1.0) * progress;
        let target_ratio = base_ratio * (1.0 / current_speed);
        let clamped_ratio = target_ratio.clamp(min_allowed_ratio, max_allowed_ratio);

        resampler
            .set_resample_ratio(clamped_ratio, true)
            .expect("set_resample_ratio");

        let chunk: Vec<Vec<f32>> = (0..channels)
            .map(|c| input_frames[c].drain(0..chunk_size).collect())
            .collect();

        let input_adapter =
            SequentialSliceOfVecs::new(chunk.as_slice(), channels, chunk_size).unwrap();
        let mut output_adapter = SequentialSliceOfVecs::new_mut(
            output_chunk.as_mut_slice(),
            channels,
            output_chunk_capacity,
        )
        .unwrap();

        let (frames_read, frames_written) = resampler
            .process_into_buffer(&input_adapter, &mut output_adapter, Some(&indexing))
            .expect("resampling failed");

        processed_frames += frames_read as u64;

        for i in 0..frames_written {
            for c in 0..channels {
                let sample = output_chunk[c][i];
                #[allow(clippy::cast_possible_truncation)]
                let sample_i16 = (sample * f32::from(i16::MAX)) as i16;
                output_audio.push(sample_i16);
            }
        }
    }

    let remaining = input_frames[0].len();
    if remaining > 0 {
        for c in 0..channels {
            let need = resampler.input_frames_next();
            let pad = need.saturating_sub(input_frames[c].len());
            input_frames[c].extend(std::iter::repeat(0.0).take(pad));
        }

        let chunk_size_actual = resampler.input_frames_next();
        let chunk: Vec<Vec<f32>> = (0..channels)
            .map(|c| input_frames[c].drain(0..chunk_size_actual).collect())
            .collect();

        indexing.partial_len = Some(remaining);

        let input_adapter =
            SequentialSliceOfVecs::new(chunk.as_slice(), channels, chunk_size_actual).unwrap();
        let mut output_adapter = SequentialSliceOfVecs::new_mut(
            output_chunk.as_mut_slice(),
            channels,
            output_chunk_capacity,
        )
        .unwrap();

        let (_frames_read, frames_written) = resampler
            .process_into_buffer(&input_adapter, &mut output_adapter, Some(&indexing))
            .expect("resampling failed");

        for i in 0..frames_written {
            for c in 0..channels {
                let sample = output_chunk[c][i];
                #[allow(clippy::cast_possible_truncation)]
                let sample_i16 = (sample * f32::from(i16::MAX)) as i16;
                output_audio.push(sample_i16);
            }
        }
    }

    output_audio
}
