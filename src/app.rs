use crate::{
    audio,
    engine::{Engine, Event, Job},
    speech::{Command, Speech},
    store::{Entry, Rule, Status, Store},
};
use eframe::egui::{self, Color32, RichText};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::{
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use uuid::Uuid;

type ImportedAudio = Result<(String, Vec<f32>), String>;
#[derive(PartialEq, Clone, Copy)]
enum Page {
    Dashboard,
    History,
    Dictionary,
    Settings,
    Guide,
}
pub struct App {
    store: Store,
    engine: Engine,
    speech: Speech,
    page: Page,
    manager: Option<GlobalHotKeyManager>,
    hotkey: Option<HotKey>,
    hotkey_edit: String,
    recorder: Option<(Uuid, audio::Recording)>,
    busy: Option<Uuid>,
    import: Option<Receiver<ImportedAudio>>,
    model: String,
    model_ready: bool,
    message: String,
    progress: String,
    selected: Option<Uuid>,
    text: String,
    devices: Vec<String>,
    last_prune: Instant,
    delete_confirm: Option<Uuid>,
    logo: egui::TextureHandle,
    output_target: Option<usize>,
    preview: Option<(std::path::PathBuf, Instant, bool)>,
    floating_job: Option<Uuid>,
    waveform: std::collections::VecDeque<f32>,
    waveform_tick: Instant,
    floating_position: Option<egui::Pos2>,
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, store: Store) -> Self {
        crate::ui::install(&cc.egui_ctx);
        cc.egui_ctx.set_theme(store.settings.theme.preference());
        let logo_image = image::load_from_memory(include_bytes!("../assets/whispering-ui.png"))
            .unwrap()
            .into_rgba8();
        let logo = cc.egui_ctx.load_texture(
            "logo",
            egui::ColorImage::from_rgba_unmultiplied(
                [logo_image.width() as usize, logo_image.height() as usize],
                logo_image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        let page = if store.settings.onboarded {
            Page::Dashboard
        } else {
            Page::Guide
        };
        let hotkey_edit = store.settings.hotkey.clone();
        let mut app = Self {
            store,
            engine: Engine::start(cc.egui_ctx.clone()),
            speech: Speech::new(),
            page,
            manager: GlobalHotKeyManager::new().ok(),
            hotkey: None,
            hotkey_edit,
            recorder: None,
            busy: None,
            import: None,
            model: "Checking bundled Parakeet v3…".into(),
            model_ready: false,
            message: String::new(),
            progress: String::new(),
            selected: None,
            text: String::new(),
            devices: audio::microphones(),
            last_prune: Instant::now(),
            delete_confirm: None,
            logo,
            output_target: None,
            preview: None,
            floating_job: None,
            waveform: std::collections::VecDeque::from(vec![0.; 24]),
            waveform_tick: Instant::now(),
            floating_position: None,
        };
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("--preview") {
            if let Some(theme) = args.get(4) {
                app.store.settings.theme = if theme == "light" {
                    crate::store::Theme::Light
                } else {
                    crate::store::Theme::Dark
                };
                cc.egui_ctx.set_theme(app.store.settings.theme.preference());
            }
            app.page = match args.get(3).map(String::as_str) {
                Some("guide") => Page::Guide,
                Some("history") => Page::History,
                Some("settings") => Page::Settings,
                Some("dictionary") => Page::Dictionary,
                _ => Page::Dashboard,
            };
            app.preview = Some((
                std::path::PathBuf::from(&args[2]).join(format!(
                    "{}-{}{}.png",
                    args.get(3).map(String::as_str).unwrap_or("dashboard"),
                    args.get(4).map(String::as_str).unwrap_or("system"),
                    if args.get(5).map(String::as_str) == Some("compact") {
                        "-compact"
                    } else {
                        ""
                    }
                )),
                Instant::now(),
                false,
            ));
            if let Some(mode) = args.get(3).filter(|s| s.starts_with("overlay-")) {
                let mut entry = Entry::new("Indicator preview".into());
                entry.status = if mode == "overlay-failed" {
                    Status::Failed
                } else {
                    Status::Transcribing
                };
                app.floating_job = Some(entry.id);
                app.store.entries.push(entry);
                if args.get(6).map(String::as_str) == Some("de") {
                    app.store.settings.language = crate::store::Language::German;
                }
            }
        }
        if let Err(e) = app.register_hotkey() {
            app.message = format!("Hotkey unavailable: {e}. Choose another in Settings.");
        }
        app
    }
    fn register_hotkey(&mut self) -> anyhow::Result<()> {
        let new: HotKey = self
            .hotkey_edit
            .trim()
            .parse()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Windows hotkey service is unavailable"))?;
        if self.hotkey.map(|h| h.id()) == Some(new.id()) {
            return Ok(());
        }
        manager.register(new)?;
        if let Some(old) = self.hotkey {
            if let Err(e) = manager.unregister(old) {
                let _ = manager.unregister(new);
                return Err(e.into());
            }
        }
        self.hotkey = Some(new);
        let previous = self.store.settings.hotkey.clone();
        self.store.settings.hotkey = self.hotkey_edit.trim().to_owned();
        if let Err(e) = self.store.save_settings() {
            let _ = manager.unregister(new);
            self.store.settings.hotkey = previous;
            self.hotkey = None;
            if let Ok(old) = self.store.settings.hotkey.parse::<HotKey>() {
                if manager.register(old).is_ok() {
                    self.hotkey = Some(old);
                }
            }
            return Err(e);
        }
        Ok(())
    }
    fn locked(&self) -> bool {
        self.busy.is_some() || self.import.is_some() || self.recorder.is_some()
    }
    fn report(&mut self, result: anyhow::Result<()>) {
        if let Err(e) = result {
            self.message = format!("{e:#}");
        }
    }
    fn transcribe(&mut self, id: Uuid) {
        if self.locked() {
            return;
        }
        let path = self.store.audio(id);
        if let Some(entry) = self.store.entries.iter_mut().find(|e| e.id == id) {
            entry.status = Status::Transcribing;
            entry.error.clear();
        }
        if let Err(e) = self.store.persist() {
            self.message = format!("Could not save job state: {e}");
            if let Some(entry) = self.store.entries.iter_mut().find(|e| e.id == id) {
                entry.status = Status::Failed;
                entry.error = self.message.clone();
            }
            return;
        }
        self.busy = Some(id);
        self.progress = "Transcribing on this computer…".into();
        self.message.clear();
        if self
            .engine
            .send
            .send(Job {
                id,
                audio: path,
                rules: self.store.settings.rules.clone(),
            })
            .is_err()
        {
            self.busy = None;
            if let Some(e) = self.store.entries.iter_mut().find(|e| e.id == id) {
                e.status = Status::Failed;
                e.error = "Transcription worker stopped. Restart Whispering and retry.".into();
            }
            self.report(self.store.persist());
        }
    }
    fn toggle_recording(&mut self) {
        if let Some((id, recording)) = self.recorder.take() {
            let recorded_seconds = recording.seconds();
            let result = recording.stop();
            if let Some(e) = self.store.entries.iter_mut().find(|e| e.id == id) {
                e.seconds = recorded_seconds;
                match &result {
                    Ok(seconds) => e.seconds = *seconds,
                    Err(err) => {
                        e.status = Status::Failed;
                        e.error = err.to_string();
                    }
                }
            }
            if let Err(e) = self.store.persist() {
                self.message = format!("Could not update history: {e}. Audio is saved on disk.");
                if let Some(entry) = self.store.entries.iter_mut().find(|e| e.id == id) {
                    entry.status = Status::Failed;
                    entry.error = self.message.clone();
                }
                return;
            }
            match result {
                Ok(_) => self.transcribe(id),
                Err(e) => self.message = e.to_string(),
            }
            return;
        }
        if self.locked() {
            self.message = "Please wait for the current transcription to finish.".into();
            return;
        }
        let entry = Entry::new("Microphone recording".into());
        let id = entry.id;
        self.output_target = if self.store.settings.type_into_app {
            crate::output::target()
        } else {
            None
        };
        if let Err(e) = self.store.add(entry) {
            self.message = e.to_string();
            return;
        }
        self.selected = Some(id);
        self.text.clear();
        match audio::Recording::start(&self.store.audio(id), &self.store.settings.microphone) {
            Ok(recording) => {
                self.floating_job = Some(id);
                self.floating_position = None;
                self.waveform = std::collections::VecDeque::from(vec![0.; 24]);
                self.recorder = Some((id, recording));
                self.message.clear();
            }
            Err(e) => {
                if let Some(entry) = self.store.entries.iter_mut().find(|e| e.id == id) {
                    entry.status = Status::Failed;
                    entry.error = format!("{e:#}");
                }
                self.message = format!("{e:#}");
                self.report(self.store.persist());
            }
        }
    }
    fn import_file(&mut self) {
        if self.locked() {
            return;
        }
        self.output_target = None;
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Choose a local audio file")
            .add_filter("Audio", &["wav", "mp3", "flac", "ogg", "m4a"])
            .pick_file()
        {
            let (tx, rx) = mpsc::channel();
            self.import = Some(rx);
            self.message.clear();
            std::thread::spawn(move || {
                let label = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let result = audio::read_audio(&path)
                    .map(|samples| (label, samples))
                    .map_err(|e| format!("{e:#}"));
                let _ = tx.send(result);
            });
        }
    }
    fn poll(&mut self) {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if self.hotkey.map(|h| h.id()) == Some(event.id) && event.state == HotKeyState::Released
            {
                self.toggle_recording();
            }
        }
        if self.recorder.as_ref().is_some_and(|(_, r)| {
            r.started.elapsed().as_secs() >= audio::MAX_SECONDS || r.error.lock().unwrap().is_some()
        }) {
            self.toggle_recording();
        }
        let imported = self.import.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Audio import stopped unexpectedly".into()))
            }
            Err(_) => None,
        });
        if let Some(result) = imported {
            self.import = None;
            match result {
                Ok((label, samples)) => {
                    let mut entry = Entry::new(label);
                    entry.seconds = samples.len() as f64 / 16000.;
                    let id = entry.id;
                    // Record metadata first so a crash during the import remains discoverable.
                    let result = self
                        .store
                        .add(entry)
                        .and_then(|_| audio::write_wav(&self.store.audio(id), &samples));
                    match result {
                        Ok(()) => {
                            self.selected = Some(id);
                            self.text.clear();
                            self.transcribe(id);
                        }
                        Err(e) => {
                            self.message = e.to_string();
                            if let Some(entry) = self.store.entries.iter_mut().find(|e| e.id == id)
                            {
                                entry.status = Status::Failed;
                                entry.error = e.to_string();
                            }
                            self.report(self.store.persist());
                        }
                    }
                }
                Err(e) => self.message = e,
            }
        }
        while let Ok(event) = self.engine.events.try_recv() {
            match event {
                Event::Model(result) => match result {
                    Ok(()) => {
                        self.model = "Parakeet v3 · ready".into();
                        self.model_ready = true;
                    }
                    Err(e) => {
                        self.model = "Model unavailable".into();
                        self.message = e;
                    }
                },
                Event::Progress(id, i, total) => {
                    if self.busy == Some(id) {
                        self.progress = format!("Transcribing section {i} of {total} locally…");
                    }
                }
                Event::Finished(id, result) => {
                    self.busy = None;
                    self.progress.clear();
                    if let Some(entry) = self.store.entries.iter_mut().find(|e| e.id == id) {
                        match result {
                            Ok(text) => {
                                entry.text = text;
                                entry.status = Status::Ready;
                                entry.error.clear();
                                self.model_ready = true;
                                self.model = "Parakeet v3 · ready".into();
                            }
                            Err(e) => {
                                entry.status = Status::Failed;
                                entry.error = e;
                                self.message="Transcription failed. Open History to retry or save the audio.".into();
                                self.page = Page::History;
                            }
                        }
                        self.selected = Some(id);
                        self.text = entry.text.clone();
                    }
                    let saved = self.store.persist();
                    if saved.is_ok()
                        && self
                            .store
                            .entries
                            .iter()
                            .any(|e| e.id == id && e.status == Status::Ready)
                    {
                        if let Some(target) = self.output_target.take() {
                            self.report(crate::output::type_text(target, &self.text));
                        }
                    }
                    self.report(saved);
                }
            }
        }
        while let Ok(e) = self.speech.errors.try_recv() {
            self.message = e;
        }
        if self.last_prune.elapsed() > Duration::from_secs(60) && !self.locked() {
            let result = self.store.prune(None);
            self.report(result);
            self.last_prune = Instant::now();
            if self
                .selected
                .is_some_and(|id| !self.store.entries.iter().any(|e| e.id == id))
            {
                self.selected = None;
                self.text.clear();
            }
        }
    }
}
#[path = "overlay.rs"]
mod overlay;
#[path = "views.rs"]
mod views;
