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

const INK: Color32 = Color32::from_rgb(34, 49, 46);
const GREEN: Color32 = Color32::from_rgb(49, 105, 84);
const MUTED: Color32 = Color32::from_rgb(104, 115, 109);
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
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, store: Store) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::light();
        style.visuals.panel_fill = Color32::from_rgb(248, 249, 245);
        style.visuals.window_fill = Color32::from_rgb(255, 255, 252);
        style.visuals.override_text_color = Some(INK);
        style.visuals.selection.bg_fill = Color32::from_rgb(211, 232, 220);
        style.visuals.selection.stroke.color = INK;
        style.spacing.item_spacing = egui::vec2(12., 12.);
        style.spacing.button_padding = egui::vec2(15., 9.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(16.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(15.));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(30.));
        cc.egui_ctx.set_style(style);
        let logo_image = image::load_from_memory(include_bytes!("../assets/whispering.png"))
            .unwrap()
            .into_rgba8();
        let logo = cc.egui_ctx.load_texture(
            "logo",
            egui::ColorImage::from_rgba_unmultiplied(
                [logo_image.width() as usize, logo_image.height() as usize],
                logo_image.as_raw(),
            ),
            Default::default(),
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
        };
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("--preview") {
            app.page = match args.get(3).map(String::as_str) {
                Some("guide") => Page::Guide,
                Some("history") => Page::History,
                Some("settings") => Page::Settings,
                _ => Page::Dashboard,
            };
            app.preview = Some((
                std::path::PathBuf::from(&args[2]).join(format!(
                    "{}.png",
                    args.get(3).map(String::as_str).unwrap_or("dashboard")
                )),
                Instant::now(),
                false,
            ));
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
                    if saved.is_ok() {
                        if let Some(target) = self.output_target.take() {
                            if self
                                .store
                                .entries
                                .iter()
                                .any(|e| e.id == id && e.status == Status::Ready)
                            {
                                self.report(crate::output::type_text(target, &self.text));
                            }
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
    fn header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.heading(title);
        ui.label(RichText::new(subtitle).color(MUTED));
        ui.add_space(18.);
    }
    fn dashboard(&mut self, ui: &mut egui::Ui) {
        Self::header(
            ui,
            "Your voice, in words.",
            "A quiet place to capture a thought. Everything is processed locally.",
        );
        egui::Frame::group(ui.style())
            .fill(Color32::WHITE)
            .inner_margin(24.)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(if self.recorder.is_some() {
                                "RECORDING"
                            } else if self.busy.is_some() {
                                "TRANSCRIBING"
                            } else {
                                "READY WHEN YOU ARE"
                            })
                            .small()
                            .color(GREEN),
                        );
                        if let Some((_, r)) = &self.recorder {
                            ui.label(
                                RichText::new(format!(
                                    "{:02}:{:02}",
                                    r.started.elapsed().as_secs() / 60,
                                    r.started.elapsed().as_secs() % 60
                                ))
                                .size(40.),
                            );
                        } else {
                            ui.label(RichText::new("Speak naturally.").size(28.));
                        }
                        ui.label(format!("{} to start or stop", self.store.settings.hotkey));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if self.recorder.is_some() {
                            "Stop & transcribe"
                        } else {
                            "Start recording"
                        };
                        if ui
                            .add_enabled(
                                self.busy.is_none() && self.import.is_none(),
                                egui::Button::new(RichText::new(label).color(Color32::WHITE))
                                    .fill(GREEN)
                                    .min_size(egui::vec2(180., 52.)),
                            )
                            .clicked()
                        {
                            self.toggle_recording();
                        }
                    });
                });
                ui.add_space(12.);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.locked(), egui::Button::new("Transcribe a file"))
                        .clicked()
                    {
                        self.import_file();
                    }
                    ui.label(
                        RichText::new("WAV, MP3, FLAC, OGG, M4A · up to 10 minutes")
                            .small()
                            .color(MUTED),
                    );
                });
            });
        ui.add_space(16.);
        let words: usize = self
            .store
            .entries
            .iter()
            .map(|e| e.text.split_whitespace().count())
            .sum();
        let seconds: f64 = self
            .store
            .entries
            .iter()
            .fold(0.0, |sum, e| sum + e.seconds);
        ui.horizontal(|ui| {
            for (value, label) in [
                (self.store.entries.len().to_string(), "saved recordings"),
                (words.to_string(), "words transcribed"),
                (format!("{:.1}", seconds / 60.), "minutes captured"),
            ] {
                egui::Frame::group(ui.style())
                    .inner_margin(14.)
                    .show(ui, |ui| {
                        ui.set_min_width(160.);
                        ui.label(RichText::new(value).size(26.));
                        ui.label(RichText::new(label).color(MUTED));
                    });
            }
        });
        ui.label(
            RichText::new("Statistics cover the recordings currently in History.")
                .small()
                .color(MUTED),
        );
        ui.add_space(16.);
        self.editor(ui);
    }
    fn editor(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Transcript / read aloud").strong());
        ui.add(
            egui::TextEdit::multiline(&mut self.text)
                .desired_width(f32::INFINITY)
                .desired_rows(7)
                .hint_text("Your transcript appears here. You can also type text to read aloud."),
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !self.text.trim().is_empty(),
                    egui::Button::new("Read aloud"),
                )
                .clicked()
                && self
                    .speech
                    .send
                    .send(Command::Speak(self.text.clone()))
                    .is_err()
            {
                self.message = "Local Windows voice is unavailable.".into();
            }
            if ui.button("Stop voice").clicked() {
                let _ = self.speech.send.send(Command::Stop);
            }
            if ui
                .add_enabled(!self.text.is_empty(), egui::Button::new("Save text…"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("transcript.txt")
                    .add_filter("Text", &["txt"])
                    .save_file()
                {
                    self.report(std::fs::write(path, &self.text).map_err(Into::into));
                }
            }
            if ui
                .add_enabled(
                    self.selected.is_some() && !self.locked(),
                    egui::Button::new("Save edits"),
                )
                .clicked()
            {
                if let Some(e) = self
                    .store
                    .entries
                    .iter_mut()
                    .find(|e| Some(e.id) == self.selected)
                {
                    e.text = self.text.clone();
                }
                self.report(self.store.persist());
            }
        });
    }
    fn history(&mut self, ui: &mut egui::Ui) {
        Self::header(
            ui,
            "A second chance.",
            "Your latest five recordings. Retry a transcription or save the original audio.",
        );
        if self.store.entries.is_empty() {
            ui.label("No recordings yet. Start a recording or import an audio file.");
        }
        let mut select = None;
        let mut retry = None;
        let mut export = None;
        let mut delete = None;
        for entry in self.store.entries.clone() {
            egui::Frame::group(ui.style())
                .inner_margin(14.)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.selected == Some(entry.id),
                                RichText::new(&entry.label).strong(),
                            )
                            .clicked()
                        {
                            select = Some(entry.id);
                        }
                        ui.label(
                            RichText::new(
                                entry
                                    .created
                                    .with_timezone(&chrono::Local)
                                    .format("%d %b · %H:%M")
                                    .to_string(),
                            )
                            .small()
                            .color(MUTED),
                        );
                        ui.label(format!("{:.0}s · {:?}", entry.seconds, entry.status));
                    });
                    if !entry.error.is_empty() {
                        ui.colored_label(Color32::from_rgb(158, 77, 46), &entry.error);
                    }
                    if !entry.text.is_empty() {
                        ui.label(entry.text.chars().take(150).collect::<String>());
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(!self.locked(), egui::Button::new("Retry transcription"))
                            .clicked()
                        {
                            retry = Some(entry.id);
                        }
                        if ui
                            .add_enabled(
                                self.recorder.as_ref().map(|r| r.0) != Some(entry.id),
                                egui::Button::new("Save audio…"),
                            )
                            .clicked()
                        {
                            export = Some(entry.id);
                        }
                        if ui
                            .add_enabled(!self.locked(), egui::Button::new("Delete"))
                            .clicked()
                        {
                            self.delete_confirm = Some(entry.id);
                        }
                        if self.delete_confirm == Some(entry.id) {
                            ui.label("Delete audio and text?");
                            if ui.button("Yes, delete").clicked() {
                                delete = Some(entry.id);
                            }
                            if ui.button("Cancel").clicked() {
                                self.delete_confirm = None;
                            }
                        }
                    });
                });
        }
        if let Some(id) = select {
            self.selected = Some(id);
            self.text = self
                .store
                .entries
                .iter()
                .find(|e| e.id == id)
                .unwrap()
                .text
                .clone();
        }
        if let Some(id) = retry {
            self.output_target = None;
            self.transcribe(id);
        }
        if let Some(id) = export {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("whispering-recording.wav")
                .add_filter("WAV audio", &["wav"])
                .save_file()
            {
                self.report(
                    std::fs::copy(self.store.audio(id), path)
                        .map(|_| ())
                        .map_err(Into::into),
                );
            }
        }
        if let Some(id) = delete {
            let result = self.store.delete(id);
            self.report(result);
            self.delete_confirm = None;
            if self.selected == Some(id) {
                self.selected = None;
                self.text.clear();
            }
        }
        ui.add_space(12.);
        self.editor(ui);
    }
    fn dictionary(&mut self, ui: &mut egui::Ui) {
        Self::header(
            ui,
            "Words, your way.",
            "Simple corrections applied after transcription. No training or online lookup.",
        );
        ui.label("Use this for names or terms that are often misheard. Matches ignore case and respect word boundaries.");
        let mut remove = None;
        egui::Grid::new("rules")
            .num_columns(3)
            .spacing([12., 12.])
            .show(ui, |ui| {
                ui.strong("When you see");
                ui.strong("Replace with");
                ui.end_row();
                for (i, rule) in self.store.settings.rules.iter_mut().enumerate() {
                    ui.add(egui::TextEdit::singleline(&mut rule.heard).desired_width(220.));
                    ui.add(egui::TextEdit::singleline(&mut rule.replacement).desired_width(220.));
                    if ui.button("Remove").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        if let Some(i) = remove {
            self.store.settings.rules.remove(i);
        }
        ui.horizontal(|ui| {
            if ui.button("Add correction").clicked() {
                self.store.settings.rules.push(Rule::default());
            }
            if ui.button("Save dictionary").clicked() {
                let result = self.store.save_settings();
                if result.is_ok() {
                    self.message = "Dictionary saved.".into();
                }
                self.report(result);
            }
        });
    }
    fn settings(&mut self, ui: &mut egui::Ui) {
        Self::header(
            ui,
            "Just the essentials.",
            "One local model. A few settings to make it yours.",
        );
        ui.strong("Recording shortcut");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.hotkey_edit);
            if ui.button("Apply shortcut").clicked() {
                match self.register_hotkey() {
                    Ok(()) => self.message = "Shortcut saved.".into(),
                    Err(e) => {
                        self.message = format!(
                        "Could not register shortcut: {e}. Your previous shortcut is still active."
                    )
                    }
                }
            }
        });
        ui.label(RichText::new("Example: Ctrl+Shift+Space or Ctrl+Alt+R. Press once to start, again to stop. Keep Whispering open or minimized.").small().color(MUTED));
        ui.checkbox(
            &mut self.store.settings.type_into_app,
            "Type completed dictation into the app where recording started",
        );
        ui.label(RichText::new("Optional output to another app, using keyboard events. No clipboard. Typing is skipped if focus changes. The destination app controls what it does with the text.").small().color(MUTED));
        ui.separator();
        ui.strong("Microphone");
        egui::ComboBox::from_id_salt("microphone")
            .selected_text(if self.store.settings.microphone.is_empty() {
                "Windows default"
            } else {
                &self.store.settings.microphone
            })
            .width(400.)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.store.settings.microphone,
                    String::new(),
                    "Windows default",
                );
                for device in &self.devices {
                    ui.selectable_value(
                        &mut self.store.settings.microphone,
                        device.clone(),
                        device,
                    );
                }
            });
        if ui.button("Refresh microphones").clicked() {
            self.devices = audio::microphones();
        }
        ui.separator();
        ui.strong("Recovery rules");
        ui.label("Keep the latest 5 recordings, including failed transcriptions.");
        egui::ComboBox::from_id_salt("retention")
            .selected_text(format!(
                "Delete after {} hours",
                self.store.settings.retention_hours
            ))
            .show_ui(ui, |ui| {
                for hours in [1, 24, 168] {
                    ui.selectable_value(
                        &mut self.store.settings.retention_hours,
                        hours,
                        format!("{hours} hours"),
                    );
                }
            });
        ui.label(RichText::new("The sixth recording replaces the oldest. Expired audio and text are deleted while the app is open and on next launch. Save anything you need before it expires.").small().color(MUTED));
        if ui.button("Save settings").clicked() {
            let result = self.store.save_settings();
            if result.is_ok() {
                self.message = "Settings saved.".into();
            }
            self.report(result);
        }
        ui.separator();
        ui.strong("Local processing");
        ui.label("Parakeet TDT 0.6B v3 · CPU · 25 languages detected automatically");
        ui.label("Read-aloud uses a locally installed Microsoft Windows voice.");
        ui.label("No accounts, cloud providers, telemetry, plugins, or automatic downloads.");
        ui.label(
            RichText::new(format!("Data folder: {}", self.store.root.display()))
                .small()
                .color(MUTED),
        );
    }
    fn guide(&mut self, ui: &mut egui::Ui) {
        Self::header(
            ui,
            "Welcome to Whispering.",
            "A little setup. Then just speak.",
        );
        for (number,title,body) in [
            ("01","Keep the app folder together","The release includes Parakeet v3 and its runtime. Extract the whole folder to a local drive, then open Whispering.exe. No model installation or account is needed."),
            ("02","Check your microphone","Choose a microphone in Settings. If access is blocked, enable desktop microphone access in Windows Settings > Privacy & security > Microphone."),
            ("03","Press, speak, press again","Use Ctrl+Shift+Space or set your own shortcut. Speak naturally, then press again to transcribe. Keep the app open or minimized. Each recording can be up to 10 minutes."),
            ("04","Keep a second chance","History retains your latest five recordings for 24 hours by default. If a transcription fails, choose Retry transcription or Save audio. Change the expiry in Settings."),
        ]{ui.horizontal_top(|ui|{ui.label(RichText::new(number).size(25.).color(GREEN));ui.vertical(|ui|{ui.strong(title);ui.label(body);});});ui.add_space(12.);}
        ui.separator();
        ui.label("Your recordings stay on this PC. Saving text or audio is an explicit export; choose a local folder if you want to avoid cloud-synced storage. The app does not use the clipboard automatically.");
        ui.label(RichText::new("Stored recordings and transcripts are ordinary local files. Use Windows device encryption to protect data at rest. Expiry is normal file deletion, not secure erasure.").small().color(MUTED));
        if ui
            .add(
                egui::Button::new(RichText::new("Start using Whispering").color(Color32::WHITE))
                    .fill(GREEN),
            )
            .clicked()
        {
            self.store.settings.onboarded = true;
            self.report(self.store.save_settings());
            self.page = Page::Dashboard;
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        ctx.request_repaint_after(Duration::from_millis(100));
        if let Some((path, started, requested)) = &mut self.preview {
            if !*requested && (self.model_ready || started.elapsed() > Duration::from_secs(30)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                *requested = true;
            }
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let rgba: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    if let Err(e) = image::save_buffer(
                        &*path,
                        &rgba,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    ) {
                        eprintln!("Preview: {e}");
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.recorder.is_some() {
            self.toggle_recording();
        }
        egui::SidePanel::left("navigation")
            .exact_width(200.)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(237, 241, 233))
                    .inner_margin(20.),
            )
            .show(ctx, |ui| {
                ui.add_space(10.);
                ui.image((self.logo.id(), egui::vec2(48., 48.)));
                ui.label(RichText::new("Whispering").size(23.).strong());
                ui.label(RichText::new("LOCAL BY NATURE").size(10.).color(MUTED));
                ui.add_space(35.);
                for (page, label) in [
                    (Page::Dashboard, "Dashboard"),
                    (Page::History, "History"),
                    (Page::Dictionary, "Dictionary"),
                    (Page::Settings, "Settings"),
                    (Page::Guide, "Getting started"),
                ] {
                    if ui
                        .add_sized(
                            [160., 40.],
                            egui::Button::new(label).selected(self.page == page),
                        )
                        .clicked()
                    {
                        self.page = page;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(RichText::new("Whispering 0.1.0").small().color(MUTED));
                    ui.label(RichText::new("On this computer").color(GREEN));
                });
            });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(if self.model_ready { GREEN } else { MUTED }, &self.model);
                if self.busy.is_some() {
                    ui.spinner();
                    ui.label(&self.progress);
                }
                if self.import.is_some() {
                    ui.spinner();
                    ui.label("Reading audio file…");
                }
                if let Some((_, r)) = &self.recorder {
                    ui.colored_label(
                        Color32::DARK_RED,
                        format!("Recording · {:.0}s", r.seconds()),
                    );
                }
            });
        });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(248, 249, 245))
                    .inner_margin(30.),
            )
            .show(ctx, |ui| {
                if !self.message.is_empty() {
                    egui::Frame::group(ui.style())
                        .fill(Color32::from_rgb(249, 239, 218))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(&self.message);
                                if ui.small_button("Dismiss").clicked() {
                                    self.message.clear();
                                }
                            });
                        });
                    ui.add_space(10.);
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    match self.page {
                        Page::Dashboard => self.dashboard(ui),
                        Page::History => self.history(ui),
                        Page::Dictionary => self.dictionary(ui),
                        Page::Settings => self.settings(ui),
                        Page::Guide => self.guide(ui),
                    }
                });
            });
    }
}
