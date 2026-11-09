//! Audio device handling: opens the default output with CPAL and runs the
//! Harpsistudio engine on the audio callback.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use hs_core::capture;
use hs_core::engine::{Cmd, Engine, Shared};
use hs_core::song::Song;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct AudioHandle {
    pub shared: Arc<Shared>,
    pub sr: u32,
    pub device: String,
    pub errors: Arc<Mutex<Option<String>>>,
    _stream: cpal::Stream,
}

pub struct RecordedAudio {
    pub name: String,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

pub struct Recorder {
    stop: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
    limit: Arc<AtomicUsize>,
    rx: Receiver<Result<RecordedAudio, String>>,
}

impl Recorder {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn progress(&self) -> f32 {
        let filled = self.buffer.try_lock().map(|b| b.len()).unwrap_or(0);
        let limit = self.limit.load(Ordering::Relaxed).max(1);
        (filled as f32 / limit as f32).clamp(0.0, 1.0)
    }

    pub fn poll(&self) -> Option<Result<RecordedAudio, String>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Err("Microphone recording stopped unexpectedly".to_string()))
            }
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The CPAL sample formats the recorder accepts. Each implementation hands the
/// raw interleaved buffer to the shared `hs_core::capture` conversion, which is
/// also what the capture-path tests exercise.
trait CaptureSample: SizedSample {
    fn downmix(data: &[Self], channels: usize) -> Vec<f32>;
}

impl CaptureSample for f32 {
    fn downmix(data: &[f32], channels: usize) -> Vec<f32> {
        capture::mono_from_f32(data, channels)
    }
}

impl CaptureSample for i16 {
    fn downmix(data: &[i16], channels: usize) -> Vec<f32> {
        capture::mono_from_i16(data, channels)
    }
}

impl CaptureSample for u16 {
    fn downmix(data: &[u16], channels: usize) -> Vec<f32> {
        capture::mono_from_u16(data, channels)
    }
}

impl CaptureSample for i32 {
    fn downmix(data: &[i32], channels: usize) -> Vec<f32> {
        capture::mono_from_i32(data, channels)
    }
}

impl CaptureSample for f64 {
    fn downmix(data: &[f64], channels: usize) -> Vec<f32> {
        capture::mono_from_f64(data, channels)
    }
}

fn build_input<T: CaptureSample>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    buffer: Arc<Mutex<Vec<f32>>>,
    stop: Arc<AtomicBool>,
    limit: usize,
    err: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, String> {
    device
        .build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _| {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                let Ok(mut buffer) = buffer.try_lock() else {
                    return;
                };
                if buffer.len() >= limit {
                    return;
                }
                let mono = T::downmix(data, channels);
                let room = limit - buffer.len();
                let take = room.min(mono.len());
                buffer.extend_from_slice(&mono[..take]);
            },
            err,
            None,
        )
        .map_err(|e| e.to_string())
}

/// Windows often lists virtual capture endpoints (Steam Streaming, internal AUX
/// jacks) ahead of real hardware, and those record pure silence. Prefer a
/// physical input, then the system default, then anything that opens at all.
fn pick_input_device(host: &cpal::Host) -> Result<(cpal::Device, String), String> {
    fn looks_virtual(name: &str) -> bool {
        let lower = name.to_lowercase();
        [
            "steam streaming",
            "virtual",
            "loopback",
            "stereo mix",
            "internal aux",
            "what u hear",
            "cable output",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
    }

    let default = host.default_input_device();
    let mut fallback: Option<(cpal::Device, String)> = None;
    let mut physical: Option<(cpal::Device, String)> = None;
    if let Ok(devices) = host.input_devices() {
        for device in devices {
            if device.default_input_config().is_err() {
                continue;
            }
            let name = device.to_string();
            if fallback.is_none() {
                fallback = Some((device.clone(), name.clone()));
            }
            if !looks_virtual(&name) {
                physical = Some((device, name));
                break;
            }
        }
    }
    if let Some(picked) = physical {
        return Ok(picked);
    }
    if let Some(device) = default {
        let name = device.to_string();
        if device.default_input_config().is_ok() {
            return Ok((device, name));
        }
    }
    fallback.ok_or_else(|| "No microphone or audio input device found".to_string())
}

fn capture(
    max_seconds: f32,
    buffer: Arc<Mutex<Vec<f32>>>,
    stop: Arc<AtomicBool>,
    limit: Arc<AtomicUsize>,
) -> Result<RecordedAudio, String> {
    let host = cpal::default_host();
    let (device, device_name) = pick_input_device(&host)?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("Cannot read input config for {device_name}: {e}"))?;
    let sample_rate = supported.sample_rate();
    let channels = supported.channels() as usize;
    let fmt = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    // The cap must follow the device rate: a 48 kHz assumption truncated 44.1 kHz
    // takes to about a second instead of the requested eight.
    let sample_limit = (sample_rate as f32 * max_seconds).ceil() as usize;
    limit.store(sample_limit, Ordering::Relaxed);
    let errors = Arc::new(Mutex::new(None));
    let err = {
        let errors = errors.clone();
        move |e: cpal::Error| {
            if let Ok(mut slot) = errors.lock() {
                *slot = Some(e.to_string());
            }
        }
    };
    let stream = match fmt {
        cpal::SampleFormat::F32 => build_input::<f32>(
            &device,
            config,
            channels,
            buffer.clone(),
            stop.clone(),
            sample_limit,
            err,
        ),
        cpal::SampleFormat::I16 => build_input::<i16>(
            &device,
            config,
            channels,
            buffer.clone(),
            stop.clone(),
            sample_limit,
            err,
        ),
        cpal::SampleFormat::U16 => build_input::<u16>(
            &device,
            config,
            channels,
            buffer.clone(),
            stop.clone(),
            sample_limit,
            err,
        ),
        cpal::SampleFormat::I32 => build_input::<i32>(
            &device,
            config,
            channels,
            buffer.clone(),
            stop.clone(),
            sample_limit,
            err,
        ),
        cpal::SampleFormat::F64 => build_input::<f64>(
            &device,
            config,
            channels,
            buffer.clone(),
            stop.clone(),
            sample_limit,
            err,
        ),
        other => {
            return Err(format!(
                "Microphone sample format {other:?} is not supported"
            ))
        }
    }?;
    stream
        .play()
        .map_err(|e| format!("Cannot start microphone: {e}"))?;

    let started = std::time::Instant::now();
    while !stop.load(Ordering::Relaxed) && started.elapsed().as_secs_f32() < max_seconds {
        let full = buffer
            .try_lock()
            .map(|b| b.len() >= sample_limit)
            .unwrap_or(false);
        if full {
            break;
        }
        // A virtual endpoint (or a muted mic) records digital near-silence.
        // Bail out early so the RECORD button can fall back to system loopback
        // instead of waiting the full eight seconds for an unusable take.
        if started.elapsed().as_secs_f32() > 1.2 {
            let rms = buffer
                .try_lock()
                .map(|b| {
                    if b.is_empty() {
                        0.0
                    } else {
                        (b.iter().map(|s| s * s).sum::<f32>() / b.len() as f32).sqrt()
                    }
                })
                .unwrap_or(0.0);
            if rms < 5e-6 {
                return Err(format!("{device_name} produced silence"));
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(40));
    drop(stream);

    if let Ok(err) = errors.lock() {
        if let Some(message) = err.as_ref() {
            return Err(format!("Microphone error: {message}"));
        }
    }
    let mut samples = buffer.lock().map(|b| b.clone()).unwrap_or_default();
    samples.truncate(sample_limit);
    if samples.len() < (sample_rate as f32 * 0.12) as usize {
        return Err("The microphone take was empty or too short".to_string());
    }
    let energy = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    if energy.sqrt() < 1e-5 {
        return Err(format!(
            "The take from {device_name} was silent — check the Windows input device and level"
        ));
    }
    Ok(RecordedAudio {
        name: "Microphone take".to_string(),
        sample_rate,
        samples,
    })
}

/// Capture the default output through WASAPI loopback. This is the fallback
/// when no physical microphone is producing signal (or none exists), and it is
/// still real audio: the bytes come from the render endpoint's stream and go
/// through the same `hs_core::capture` conversion as the mic path.
#[cfg(windows)]
fn capture_system(
    max_seconds: f32,
    buffer: Arc<Mutex<Vec<f32>>>,
    stop: Arc<AtomicBool>,
    limit: Arc<AtomicUsize>,
) -> Result<RecordedAudio, String> {
    use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

    let _ = wasapi::initialize_mta();
    let enumerator =
        DeviceEnumerator::new().map_err(|e| format!("Cannot enumerate audio devices: {e}"))?;
    let device = enumerator
        .get_default_device(Direction::Render)
        .map_err(|e| format!("Cannot open the default output for loopback: {e}"))?;
    let device_name = device
        .get_friendlyname()
        .unwrap_or_else(|_| "System output".to_string());
    let mut client = device
        .get_iaudioclient()
        .map_err(|e| format!("Cannot open loopback client: {e}"))?;

    let rate = 48_000u32;
    let channels = 2usize;
    let format = WaveFormat::new(32, 32, SampleType::Float, rate as usize, channels, None);
    let blockalign = format.get_blockalign() as usize;
    let (_, min_time) = client.get_device_period().unwrap_or((0, 200_000));
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_time.max(100_000),
    };
    client
        .initialize_client(&format, Direction::Capture, mode, None)
        .map_err(|e| format!("Cannot start loopback capture on {device_name}: {e}"))?;
    let event = client
        .set_get_eventhandle()
        .map_err(|e| format!("Cannot create loopback event: {e}"))?;
    let capture_client = client
        .get_audiocaptureclient()
        .map_err(|e| format!("Cannot read loopback stream: {e}"))?;
    client
        .start_stream()
        .map_err(|e| format!("Cannot start loopback stream: {e}"))?;

    let sample_limit = (rate as f32 * max_seconds).ceil() as usize;
    limit.store(sample_limit, Ordering::Relaxed);
    let mut data = vec![0u8; blockalign * 4096];
    let started = std::time::Instant::now();
    'outer: while !stop.load(Ordering::Relaxed) && started.elapsed().as_secs_f32() < max_seconds {
        if event.wait_for_event(200).is_err() {
            break;
        }
        loop {
            let frames = match capture_client.get_next_packet_size() {
                Ok(Some(frames)) if frames > 0 => frames as usize,
                Ok(_) => break,
                Err(_) => break 'outer,
            };
            let bytes = frames * blockalign;
            if data.len() < bytes {
                data.resize(bytes, 0);
            }
            let read = match capture_client.read_from_device(&mut data[..bytes]) {
                Ok((read, _)) => read as usize,
                Err(_) => break 'outer,
            };
            let floats: Vec<f32> = data[..read * blockalign]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_le_bytes(*c))
                .collect();
            let mono = capture::mono_from_f32(&floats, channels);
            let Ok(mut buffer) = buffer.lock() else {
                break 'outer;
            };
            let room = sample_limit.saturating_sub(buffer.len());
            let take = room.min(mono.len());
            buffer.extend_from_slice(&mono[..take]);
            if buffer.len() >= sample_limit {
                break 'outer;
            }
        }
    }
    let _ = client.stop_stream();

    let mut samples = buffer.lock().map(|b| b.clone()).unwrap_or_default();
    samples.truncate(sample_limit);
    if samples.len() < (rate as f32 * 0.12) as usize {
        return Err("The system loopback take was empty or too short".to_string());
    }
    let energy = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    if energy.sqrt() < 1e-6 {
        return Err(format!(
            "System loopback from {device_name} was silent — is anything playing?"
        ));
    }
    Ok(RecordedAudio {
        name: "System loopback take".to_string(),
        sample_rate: rate,
        samples,
    })
}

#[cfg(not(windows))]
fn capture_system(
    _max_seconds: f32,
    _buffer: Arc<Mutex<Vec<f32>>>,
    _stop: Arc<AtomicBool>,
    _limit: Arc<AtomicUsize>,
) -> Result<RecordedAudio, String> {
    Err("System loopback capture is only available on Windows".to_string())
}

/// Record a mono 8-second vocal take (or stop early), delivered through
/// `Recorder::poll`. Input is opened on a worker thread so the UI never stalls.
pub fn start_recording(max_seconds: f32) -> Result<Recorder, String> {
    let max_seconds = max_seconds.clamp(1.0, 30.0);
    let stop = Arc::new(AtomicBool::new(false));
    let buffer = Arc::new(Mutex::new(Vec::new()));
    // Replaced with the device's real rate as soon as the input stream opens;
    // this estimate only covers the first UI frame.
    let limit = Arc::new(AtomicUsize::new((max_seconds * 48_000.0) as usize));
    let (tx, rx) = channel();
    let worker_stop = stop.clone();
    let worker_buffer = buffer.clone();
    let worker_limit = limit.clone();
    std::thread::spawn(move || {
        let result = match capture(
            max_seconds,
            worker_buffer.clone(),
            worker_stop.clone(),
            worker_limit.clone(),
        ) {
            Ok(take) => Ok(take),
            Err(mic_error) => capture_system(max_seconds, worker_buffer, worker_stop, worker_limit)
                .map_err(|system_error| {
                    format!("{mic_error}; system loopback failed: {system_error}")
                }),
        };
        let _ = tx.send(result);
    });
    Ok(Recorder {
        stop,
        buffer,
        limit,
        rx,
    })
}

fn err_cb(slot: Arc<Mutex<Option<String>>>) -> impl FnMut(cpal::Error) + Send + 'static {
    move |e| {
        if let Ok(mut g) = slot.lock() {
            *g = Some(e.to_string());
        }
    }
}

/// Stream format used when building the output callback.
#[derive(Clone, Copy)]
struct Format {
    sr: u32,
    channels: usize,
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    format: Format,
    song: Arc<Song>,
    rx: Receiver<Cmd>,
    shared: Arc<Shared>,
    err: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let mut engine = Engine::new(format.sr as f32, song, Some(rx), Some(shared));
    let mut scratch: Vec<f32> = vec![0.0; 8192];
    device
        .build_output_stream::<T, _, _>(
            config,
            move |data: &mut [T], _| {
                let n = data.len();
                if scratch.len() < n {
                    scratch.resize(n, 0.0);
                }
                engine.process(&mut scratch[..n], format.channels);
                for (dst, src) in data.iter_mut().zip(scratch.iter()) {
                    *dst = T::from_sample(*src);
                }
            },
            err,
            None,
        )
        .map_err(|e| e.to_string())
}

pub fn start(song: &Song) -> Result<AudioHandle, String> {
    let (shared, song_arc, rx) = Shared::new(song);
    let errors = Arc::new(Mutex::new(None));
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No audio output device found".to_string())?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("Cannot read output config: {e}"))?;
    let sr = supported.sample_rate();
    let channels = supported.channels() as usize;
    let fmt = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let format = Format { sr, channels };
    shared.sample_rate.store(sr, Ordering::Relaxed);
    let device_name = format!("System output · {} ch", channels);

    let stream = match fmt {
        cpal::SampleFormat::F32 => build::<f32>(
            &device,
            config,
            format,
            song_arc,
            rx,
            shared.clone(),
            err_cb(errors.clone()),
        ),
        cpal::SampleFormat::I16 => build::<i16>(
            &device,
            config,
            format,
            song_arc,
            rx,
            shared.clone(),
            err_cb(errors.clone()),
        ),
        cpal::SampleFormat::U16 => build::<u16>(
            &device,
            config,
            format,
            song_arc,
            rx,
            shared.clone(),
            err_cb(errors.clone()),
        ),
        other => return Err(format!("Output sample format {other:?} is not supported")),
    }?;
    stream
        .play()
        .map_err(|e| format!("Cannot start audio: {e}"))?;

    Ok(AudioHandle {
        shared,
        sr,
        device: device_name,
        errors,
        _stream: stream,
    })
}
