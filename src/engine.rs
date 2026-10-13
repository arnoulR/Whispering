use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
};
use transcribe_rs::onnx::{
    parakeet::{ParakeetModel, ParakeetParams},
    Quantization,
};
use uuid::Uuid;

pub struct Job {
    pub id: Uuid,
    pub audio: PathBuf,
    pub rules: Vec<crate::store::Rule>,
}
pub enum Event {
    Model(Result<(), String>),
    Progress(Uuid, usize, usize),
    Finished(Uuid, Result<String, String>),
}
pub struct Engine {
    pub send: Sender<Job>,
    pub events: Receiver<Event>,
}
pub fn bundle_root() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(Path::new("."))
        .to_owned()
}
pub fn verify_model(root: &Path) -> Result<()> {
    let hashes: BTreeMap<String, String> = serde_json::from_str(
        include_str!("../assets/model-hashes.json").trim_start_matches('\u{feff}'),
    )?;
    for (name, expected) in hashes {
        let path = root.join("models/parakeet-v3").join(&name);
        let mut file=File::open(&path).with_context(||format!("Bundled Parakeet v3 is missing {name}. Extract the complete Whispering release folder."))?;
        let mut hash = Sha256::new();
        let mut buf = vec![0; 1024 * 1024];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
        }
        if format!("{:x}", hash.finalize()) != expected {
            bail!("Model integrity check failed for {name}. Restore the original release bundle.");
        }
    }
    Ok(())
}
fn load() -> Result<ParakeetModel> {
    let root = bundle_root();
    verify_model(&root)?;
    let dll = root.join("onnxruntime.dll");
    if !dll.is_file() {
        bail!("Bundled ONNX runtime is missing. Extract the complete release folder.");
    }
    let runtime_hash = format!("{:x}", Sha256::digest(std::fs::read(&dll)?));
    if runtime_hash != "114947d633e6844ce3c4b51ef6678f776628571d08a5763859c61642c8dcca9c" {
        bail!("ONNX runtime integrity check failed. Restore the original release bundle.");
    }
    let environment = ort::init_from(dll)?;
    // Share a bounded pool across all three sessions and avoid burning CPU while idle.
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .min(4);
    let pool = ort::environment::GlobalThreadPoolOptions::default()
        .with_intra_threads(threads)?
        .with_inter_threads(1)?
        .with_spin_control(false)?;
    environment
        .with_name("Whispering")
        .with_telemetry(false)
        .with_global_thread_pool(pool)
        .commit();
    transcribe_rs::set_ort_accelerator(transcribe_rs::OrtAccelerator::CpuOnly);
    ParakeetModel::load(&root.join("models/parakeet-v3"), &Quantization::Int8)
        .map_err(|e| anyhow::anyhow!(e))
}
// Split at the quietest 100 ms window near a 30 s boundary to bound RAM use.
fn chunks(samples: &[f32]) -> Vec<&[f32]> {
    let mut parts = vec![];
    let mut start = 0;
    while start < samples.len() {
        let end = if samples.len() - start <= 35 * 16000 {
            samples.len()
        } else {
            (start + 25 * 16000..start + 30 * 16000)
                .step_by(1600)
                .min_by(|&a, &b| {
                    let energy = |i: usize| samples[i..i + 1600].iter().map(|s| s * s).sum::<f32>();
                    energy(a).total_cmp(&energy(b))
                })
                .unwrap()
                + 800
        };
        parts.push(&samples[start..end]);
        start = end;
    }
    parts
}
fn run(
    model: &mut ParakeetModel,
    path: &Path,
    mut progress: impl FnMut(usize, usize),
) -> Result<String> {
    let samples = crate::audio::read_audio(path)?;
    let parts = chunks(&samples);
    let total = parts.len();
    let mut text = vec![];
    for (i, part) in parts.into_iter().enumerate() {
        progress(i + 1, total);
        text.push(
            model
                .transcribe_with(part, &ParakeetParams::default())?
                .text
                .trim()
                .to_string(),
        );
    }
    let text = text.join(" ").trim().to_owned();
    if text.is_empty() {
        bail!("No speech was detected. Your audio is saved; retry or export it from History.");
    }
    Ok(text)
}
impl Engine {
    pub fn start(ctx: eframe::egui::Context) -> Self {
        let (send, jobs) = mpsc::channel::<Job>();
        let (events_tx, events) = mpsc::channel();
        std::thread::spawn(move || {
            let loaded=std::panic::catch_unwind(load).unwrap_or_else(|_|Err(anyhow::anyhow!("The local engine could not initialize. Restore the release bundle and restart.")));
            let mut model = match loaded {
                Ok(m) => {
                    let _ = events_tx.send(Event::Model(Ok(())));
                    Some(m)
                }
                Err(e) => {
                    let _ = events_tx.send(Event::Model(Err(format!("{e:#}"))));
                    None
                }
            };
            ctx.request_repaint();
            for job in jobs {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<String> {
                        if model.is_none() {
                            model = Some(load()?);
                        }
                        let text = run(model.as_mut().unwrap(), &job.audio, |i, total| {
                            let _ = events_tx.send(Event::Progress(job.id, i, total));
                            ctx.request_repaint();
                        })?;
                        Ok(crate::store::apply_rules(&text, &job.rules))
                    }))
                    .unwrap_or_else(|_| {
                        model = None;
                        Err(anyhow::anyhow!(
                            "Transcription stopped unexpectedly. Audio is safe; try again."
                        ))
                    });
                let _ = events_tx.send(Event::Finished(
                    job.id,
                    result.map_err(|e| format!("{e:#}")),
                ));
                ctx.request_repaint();
            }
        });
        Self { send, events }
    }
}
pub fn transcribe_test(path: &Path) -> Result<String> {
    run(&mut load()?, path, |_, _| {})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunking_loses_no_samples_and_bounds_memory() {
        let input = vec![0.1; 96 * 16000];
        let out = chunks(&input);
        assert_eq!(out.iter().map(|p| p.len()).sum::<usize>(), input.len());
        assert!(out.iter().all(|p| p.len() <= 35 * 16000));
    }
}
