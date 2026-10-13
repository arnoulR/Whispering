use anyhow::{bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    fs::File,
    path::Path,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

pub const MAX_SECONDS: u64 = 600;
pub fn microphones() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|d| d.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}
pub struct Recording {
    stream: Option<cpal::Stream>,
    writer: Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<File>>>>>,
    pub error: Arc<Mutex<Option<String>>>,
    samples: Arc<AtomicU64>,
    level: Arc<AtomicU32>,
    rate: u32,
    pub started: Instant,
}
impl Recording {
    pub fn start(path: &Path, device_name: &str) -> Result<Self> {
        let host = cpal::default_host();
        let device = if device_name.is_empty() {
            host.default_input_device()
        } else {
            host.input_devices()?
                .find(|d| d.name().ok().as_deref() == Some(device_name))
        }
        .context("Microphone not available. Choose an available microphone in Settings.")?;
        let supported = device.default_input_config()?;
        let config: cpal::StreamConfig = supported.clone().into();
        let rate = config.sample_rate.0;
        let writer = Arc::new(Mutex::new(Some(hound::WavWriter::create(
            path,
            hound::WavSpec {
                channels: 1,
                sample_rate: rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )?)));
        let error = Arc::new(Mutex::new(None));
        let samples = Arc::new(AtomicU64::new(0));
        let level = Arc::new(AtomicU32::new(0));
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(
                &device,
                &config,
                writer.clone(),
                error.clone(),
                samples.clone(),
                level.clone(),
            )?,
            cpal::SampleFormat::I16 => build::<i16>(
                &device,
                &config,
                writer.clone(),
                error.clone(),
                samples.clone(),
                level.clone(),
            )?,
            cpal::SampleFormat::U16 => build::<u16>(
                &device,
                &config,
                writer.clone(),
                error.clone(),
                samples.clone(),
                level.clone(),
            )?,
            other => bail!("Unsupported microphone sample format: {other:?}"),
        };
        stream.play()?;
        Ok(Self {
            stream: Some(stream),
            writer,
            error,
            samples,
            level,
            rate,
            started: Instant::now(),
        })
    }
    pub fn seconds(&self) -> f64 {
        self.samples.load(Ordering::Relaxed) as f64 / self.rate as f64
    }
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.swap(0, Ordering::Relaxed))
    }
    pub fn stop(mut self) -> Result<f64> {
        self.stream.take();
        if let Some(w) = self.writer.lock().unwrap().take() {
            w.finalize()?;
        }
        if let Some(e) = self.error.lock().unwrap().as_ref() {
            bail!("{e}. The saved portion remains in History.");
        }
        Ok(self.seconds())
    }
}
impl Drop for Recording {
    fn drop(&mut self) {
        self.stream.take();
        if let Ok(mut w) = self.writer.lock() {
            if let Some(w) = w.take() {
                let _ = w.finalize();
            }
        }
    }
}
type Writer = Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<File>>>>>;
fn build<T: cpal::SizedSample + cpal::Sample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    writer: Writer,
    error: Arc<Mutex<Option<String>>>,
    count: Arc<AtomicU64>,
    level: Arc<AtomicU32>,
) -> Result<cpal::Stream>
where
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    let limit = config.sample_rate.0 as u64 * MAX_SECONDS;
    let err_stream = error.clone();
    Ok(device.build_input_stream(
        config,
        move |data: &[T], _| {
            // Positive float bits preserve ordering, so retain peaks between UI frames.
            let peak = data
                .iter()
                .map(|s| s.to_sample::<f32>().abs())
                .filter(|s| s.is_finite())
                .fold(0.0_f32, f32::max)
                .min(1.0);
            level.fetch_max(peak.to_bits(), Ordering::Relaxed);
            if error.lock().unwrap().is_some() {
                return;
            }
            if let Some(w) = writer.lock().unwrap().as_mut() {
                let result = (|| -> Result<()> {
                    for frame in data.chunks_exact(channels) {
                        if count.load(Ordering::Relaxed) >= limit {
                            break;
                        }
                        let mono = frame.iter().map(|s| s.to_sample::<f32>()).sum::<f32>()
                            / channels as f32;
                        w.write_sample((mono.clamp(-1., 1.) * i16::MAX as f32) as i16)?;
                        count.fetch_add(1, Ordering::Relaxed);
                    }
                    // Hound updates the WAV header on flush, keeping the file readable after a process crash.
                    w.flush()?;
                    Ok(())
                })();
                if let Err(e) = result {
                    *error.lock().unwrap() = Some(format!("Audio could not be saved: {e}"));
                }
            }
        },
        move |e| {
            *err_stream.lock().unwrap() = Some(format!("Microphone stopped: {e}"));
        },
        None,
    )?)
}

pub fn read_audio(path: &Path) -> Result<Vec<f32>> {
    use symphonia::core::{
        audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
        meta::MetadataOptions, probe::Hint,
    };
    let source = MediaSourceStream::new(Box::new(File::open(path)?), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?
        .format;
    let track = format
        .default_track()
        .context("No audio track in this file")?;
    let id = track.id;
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    let mut mono = Vec::new();
    let mut rate = 0;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .context("Could not decode this audio file")?;
        let spec = *decoded.spec();
        if rate != 0 && rate != spec.rate {
            bail!("Changing sample rates are not supported");
        }
        rate = spec.rate;
        let channels = spec.channels.count();
        if channels == 0 || rate == 0 {
            bail!("Invalid audio format");
        }
        let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buf.copy_interleaved_ref(decoded);
        mono.extend(
            buf.samples()
                .chunks_exact(channels)
                .map(|c| c.iter().sum::<f32>() / channels as f32),
        );
        if mono.len() as u64 > rate as u64 * MAX_SECONDS {
            bail!("Please use audio of 10 minutes or less.");
        }
    }
    if mono.is_empty() {
        bail!("This file contains no audio");
    }
    Ok(resample(&mono, rate, 16000))
}
pub fn write_wav(path: &Path, samples: &[f32]) -> Result<()> {
    let mut w = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    for sample in samples {
        w.write_sample((sample.clamp(-1., 1.) * 32767.) as i16)?;
    }
    w.finalize()?;
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)?
        .sync_all()?;
    Ok(())
}
fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let cutoff = (to as f64 / from as f64).min(1.) * 0.94;
    (0..(input.len() as f64 / ratio) as usize)
        .map(|i| {
            let position = i as f64 * ratio;
            let center = position.floor() as isize;
            let mut value = 0.;
            let mut total = 0.;
            for tap in -32..=32 {
                let j = center + tap;
                if j < 0 || j >= input.len() as isize {
                    continue;
                }
                let distance = position - j as f64;
                let x = std::f64::consts::PI * distance * cutoff;
                let sinc = if x.abs() < 1e-9 { 1. } else { x.sin() / x };
                let window = 0.5 + 0.5 * (std::f64::consts::PI * distance / 33.).cos();
                let weight = sinc * window;
                value += input[j as usize] as f64 * weight;
                total += weight;
            }
            (value / total) as f32
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flushed_recording_is_readable_before_finalize() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("in-progress.wav");
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 16000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..1600 {
            writer.write_sample(100i16).unwrap();
        }
        writer.flush().unwrap();
        assert_eq!(read_audio(&path).unwrap().len(), 1600);
    }
    #[test]
    fn downsample_preserves_duration_and_dc() {
        let y = resample(&vec![0.5; 48000], 48000, 16000);
        assert_eq!(y.len(), 16000);
        assert!(y.iter().all(|v| (v - 0.5).abs() < 0.001));
    }
    #[test]
    fn wav_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.wav");
        write_wav(&p, &vec![0.2; 16000]).unwrap();
        let v = read_audio(&p).unwrap();
        assert_eq!(v.len(), 16000);
        assert!((v[100] - 0.2).abs() < 0.001);
    }
}
