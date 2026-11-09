//! Imported audio takes. Samples are stored as mono PCM16 and serialized as
//! base64 inside the project JSON, so a project never depends on the original
//! WAV path.

use crate::samples;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioTake {
    pub name: String,
    pub sample_rate: u32,
    #[serde(with = "pcm16_base64")]
    pub samples: Vec<i16>,
}

impl AudioTake {
    pub fn from_wav_file(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Could not read WAV: {e}"))?;
        let sample = samples::decode_wav(&bytes)
            .ok_or_else(|| "Unsupported or damaged WAV file".to_string())?;
        if sample.data.len() < 64 {
            return Err("The WAV is too short to use as a take".to_string());
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported vocal")
            .to_string();
        Ok(Self::from_samples(
            name,
            sample.rate.round().max(1.0) as u32,
            &sample.data,
        ))
    }

    pub fn from_samples(name: impl Into<String>, sample_rate: u32, samples: &[f32]) -> Self {
        let samples = samples
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i16)
            .collect();
        Self {
            name: name.into(),
            sample_rate: sample_rate.max(1),
            samples,
        }
    }

    pub fn duration_seconds(&self) -> f32 {
        self.samples.len() as f32 / self.sample_rate.max(1) as f32
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode_base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc = 0u32;
    let mut bits = 0u8;
    for byte in text.bytes() {
        if byte == b'=' {
            break;
        }
        let value = B64.iter().position(|b| *b == byte)? as u32;
        acc = (acc << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

mod pcm16_base64 {
    use super::*;

    pub fn serialize<S: Serializer>(samples: &[i16], serializer: S) -> Result<S::Ok, S::Error> {
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        serializer.serialize_str(&encode_base64(&bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<i16>, D::Error> {
        let text = String::deserialize(deserializer)?;
        let bytes = decode_base64(&text)
            .ok_or_else(|| serde::de::Error::custom("invalid audio take base64"))?;
        if bytes.len() % 2 != 0 {
            return Err(serde::de::Error::custom("odd audio take byte count"));
        }
        Ok(bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_round_trips_through_project_json() {
        let take = AudioTake::from_samples("test", 48_000, &[0.0, 0.5, -0.5, 0.999, -0.999]);
        let json = serde_json::to_string(&take).unwrap();
        let loaded: AudioTake = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, take);
        assert_eq!(loaded.samples.len(), 5);
    }
}
