//! Microphone capture conversion.
//!
//! CPAL hands the recorder an interleaved buffer in the device's native sample
//! format. These functions turn that buffer into the mono f32 stream stored in
//! an [`crate::audio_take::AudioTake`]. The recorder and the capture-path tests
//! both call them, so the conversion exercised by tests is exactly the one used
//! on real device audio.

fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(interleaved.len() / channels);
    for frame in interleaved.chunks_exact(channels) {
        let sum: f32 = frame.iter().sum();
        out.push(sum / channels as f32);
    }
    out
}

/// Convert a CPAL `f32` input buffer to mono.
pub fn mono_from_f32(interleaved: &[f32], channels: usize) -> Vec<f32> {
    downmix(interleaved, channels)
}

/// Convert a CPAL `i16` input buffer to mono (the scale CPAL uses: `i16::MIN`
/// maps to -1.0, `i16::MAX` to just under +1.0).
pub fn mono_from_i16(interleaved: &[i16], channels: usize) -> Vec<f32> {
    let scaled: Vec<f32> = interleaved.iter().map(|s| *s as f32 / 32_768.0).collect();
    downmix(&scaled, channels)
}

/// Convert a CPAL `u16` input buffer to mono. Unsigned PCM is centred on
/// 32768, so that value becomes silence.
pub fn mono_from_u16(interleaved: &[u16], channels: usize) -> Vec<f32> {
    let scaled: Vec<f32> = interleaved
        .iter()
        .map(|s| (*s as f32 - 32_768.0) / 32_768.0)
        .collect();
    downmix(&scaled, channels)
}

/// Convert a CPAL `i32` input buffer to mono.
pub fn mono_from_i32(interleaved: &[i32], channels: usize) -> Vec<f32> {
    let scaled: Vec<f32> = interleaved
        .iter()
        .map(|s| *s as f32 / 2_147_483_648.0)
        .collect();
    downmix(&scaled, channels)
}

/// Convert a CPAL `f64` input buffer to mono.
pub fn mono_from_f64(interleaved: &[f64], channels: usize) -> Vec<f32> {
    let scaled: Vec<f32> = interleaved.iter().map(|s| *s as f32).collect();
    downmix(&scaled, channels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_is_averaged_and_trailing_partial_frame_is_ignored() {
        let mono = mono_from_f32(&[0.5, -0.5, 0.25, 0.25, 9.0], 2);
        assert_eq!(mono, vec![0.0, 0.25]);
    }

    #[test]
    fn mono_passes_through_and_signed_formats_scale() {
        assert_eq!(mono_from_f32(&[0.1, -0.2], 1), vec![0.1, -0.2]);
        assert!((mono_from_i16(&[16_384, -16_384], 1)[0] - 0.5).abs() < 1e-4);
        assert!(mono_from_u16(&[32_768, 32_768], 1)[0].abs() < 1e-6);
        assert!((mono_from_i32(&[1_073_741_824, 0], 1)[0] - 0.5).abs() < 1e-4);
        assert!((mono_from_f64(&[0.75, 0.75], 1)[0] - 0.75).abs() < 1e-6);
    }

    #[test]
    fn zero_channels_is_empty_instead_of_panicking() {
        assert!(mono_from_f32(&[0.5], 0).is_empty());
    }
}
