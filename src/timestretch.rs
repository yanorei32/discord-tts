use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

const TARGET_SPEED: f64 = 3.0;
const RAMP_DURATION_SEC: f64 = 20.0;
const INITIAL_DELAY_SEC: f64 = 10.0;
const CHUNK_SIZE: usize = 1024;

/// Applies time-stretching acceleration to the input audio.
pub fn apply_time_stretch(
    input_samples: &[i16],
    channels: usize,
    input_sample_rate: u32,
) -> Vec<i16> {
    // 1. Setup Resampler
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let base_ratio = 1.0;

    // Derived constants based on sample logic, adjusted for base_ratio
    let max_relative_ratio = TARGET_SPEED * 1.1;

    let mut resampler =
        SincFixedIn::<f32>::new(base_ratio, max_relative_ratio, params, CHUNK_SIZE, channels)
            .expect("failed to create resampler");

    // Limits for ratio
    let min_allowed_ratio = base_ratio / max_relative_ratio;
    let max_allowed_ratio = base_ratio * max_relative_ratio;

    // Buffers
    let mut input_frames: Vec<Vec<f32>> = vec![vec![]; channels];
    let mut output_audio: Vec<i16> = Vec::new();

    // De-interleave
    for (i, &sample) in input_samples.iter().enumerate() {
        let sample_f32 = f32::from(sample) / f32::from(i16::MAX);
        input_frames[i % channels].push(sample_f32);
    }

    let mut processed_frames: u64 = 0;

    // Process loop
    while input_frames[0].len() >= CHUNK_SIZE {
        // `chunk_size` is the input chunk size for SincFixedIn.
        #[allow(clippy::cast_precision_loss)]
        let processed_seconds = processed_frames as f64 / f64::from(input_sample_rate);

        let progress = if processed_seconds < INITIAL_DELAY_SEC {
            0.0
        } else {
            ((processed_seconds - INITIAL_DELAY_SEC) / RAMP_DURATION_SEC).min(1.0)
        };

        let current_speed = 1.0 + (TARGET_SPEED - 1.0) * progress;

        // Target ratio logic from sample: 1.0 / current_speed
        // Modified by base_ratio for SR conversion
        let target_ratio = base_ratio * (1.0 / current_speed);

        // Clamp ratio to be safe for SincFixedIn
        let clamped_ratio = target_ratio.clamp(min_allowed_ratio, max_allowed_ratio);

        resampler.set_resample_ratio(clamped_ratio, true).unwrap();

        let mut chunk = vec![vec![0.0; CHUNK_SIZE]; channels];
        for c in 0..channels {
            let part: Vec<f32> = input_frames[c].drain(0..CHUNK_SIZE).collect();
            chunk[c] = part;
        }

        let resampled_output = resampler.process(&chunk, None).expect("resampling failed");
        processed_frames += CHUNK_SIZE as u64;

        // Interleave result
        if !resampled_output.is_empty() {
            for i in 0..resampled_output[0].len() {
                for channel_data in &resampled_output {
                    let sample = channel_data[i];
                    #[allow(clippy::cast_possible_truncation)]
                    let sample = (sample * f32::from(i16::MAX)) as i16;
                    output_audio.push(sample);
                }
            }
        }
    }

    // Flush remaining
    let remaining = input_frames[0].len();
    if remaining > 0 {
        let padding = CHUNK_SIZE - remaining;
        for channel_buffer in &mut input_frames {
            channel_buffer.extend(std::iter::repeat_n(0.0, padding));
        }

        let mut chunk = vec![vec![0.0; CHUNK_SIZE]; channels];
        for c in 0..channels {
            let part: Vec<f32> = input_frames[c].drain(0..CHUNK_SIZE).collect();
            chunk[c] = part;
        }

        let resampled_output = resampler.process(&chunk, None).expect("resampling failed");

        for i in 0..resampled_output[0].len() {
            for channel_data in &resampled_output {
                let sample = channel_data[i];
                #[allow(clippy::cast_possible_truncation)]
                let sample = (sample * f32::from(i16::MAX)) as i16;
                output_audio.push(sample);
            }
        }
    }

    output_audio
}
