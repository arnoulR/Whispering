use super::*;
use crate::{
    store::Theme,
    ui::{self as design, Icon, Palette},
};
use egui::{pos2, vec2, Align2, FontId, Rect, Stroke};

impl App {
    fn theme_switch(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        egui::Frame::new()
            .fill(p.raised)
            .corner_radius(10)
            .inner_margin(3.)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 3.;
                ui.allocate_ui_with_layout(
                    vec2(120., 34.),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        for (theme, symbol, label) in [
                            (Theme::System, Icon::Monitor, "Follow Windows"),
                            (Theme::Light, Icon::Sun, "Light mode"),
                            (Theme::Dark, Icon::Moon, "Dark mode"),
                        ] {
                            if design::icon_button(
                                ui,
                                symbol,
                                label,
                                self.store.settings.theme == theme,
                            )
                            .clicked()
                            {
                                self.store.settings.theme = theme;
                                ui.ctx().set_theme(theme.preference());
                                self.report(self.store.save_settings());
                            }
                        }
                    },
                );
            });
    }
    fn title(ui: &mut egui::Ui, title: &str) {
        ui.heading(RichText::new(title).size(29.).strong());
    }
    fn dashboard(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        ui.horizontal(|ui| {
            Self::title(ui, "Dictation");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.locked(), design::secondary("Import audio"))
                    .on_hover_text("WAV, MP3, FLAC, OGG or M4A · up to 60 minutes")
                    .clicked()
                {
                    self.import_file();
                }
            });
        });
        ui.add_space(22.);
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 216.), egui::Sense::hover());
        let (left, right) = if p.dark {
            (Color32::from_rgb(22, 42, 80), Color32::from_rgb(14, 23, 41))
        } else {
            (
                Color32::from_rgb(227, 237, 255),
                Color32::from_rgb(239, 244, 255),
            )
        };
        design::gradient(ui.painter(), rect, 16., left, right);
        ui.painter().rect_stroke(
            rect,
            16,
            Stroke::new(
                1.0_f32,
                if p.dark {
                    Color32::from_rgb(39, 60, 96)
                } else {
                    Color32::from_rgb(210, 223, 247)
                },
            ),
            egui::StrokeKind::Inside,
        );
        let title = if self.recorder.is_some() {
            "Listening…"
        } else if self.busy.is_some() {
            "Transcribing…"
        } else if self.import.is_some() {
            "Importing audio…"
        } else {
            "Ready to record"
        };
        ui.painter().text(
            rect.min + vec2(28., 27.),
            Align2::LEFT_TOP,
            title,
            FontId::proportional(27.),
            p.ink,
        );
        let mut key_x = rect.left() + 28.;
        for key in self.store.settings.hotkey.split('+') {
            let width = key.len() as f32 * 7.2 + 20.;
            let key_rect = Rect::from_min_size(pos2(key_x, rect.top() + 77.), vec2(width, 27.));
            ui.painter().rect_filled(
                key_rect,
                5,
                if p.dark {
                    Color32::from_rgb(31, 50, 80)
                } else {
                    Color32::WHITE
                },
            );
            ui.painter().text(
                key_rect.center(),
                Align2::CENTER_CENTER,
                key,
                FontId::proportional(12.),
                p.muted,
            );
            key_x += width + 7.;
        }
        if let Some((_, r)) = &self.recorder {
            ui.painter().text(
                rect.min + vec2(28., 118.),
                Align2::LEFT_TOP,
                format!(
                    "{:02}:{:02}",
                    r.started.elapsed().as_secs() / 60,
                    r.started.elapsed().as_secs() % 60
                ),
                FontId::monospace(17.),
                p.accent,
            );
        } else if self.busy.is_some() {
            ui.painter().text(
                rect.min + vec2(28., 118.),
                Align2::LEFT_TOP,
                &self.progress,
                FontId::proportional(12.),
                p.muted,
            );
        }
        let button_rect = Rect::from_min_size(rect.min + vec2(28., 151.), vec2(177., 42.));
        ui.scope_builder(egui::UiBuilder::new().max_rect(button_rect), |ui| {
            if ui
                .add_enabled(
                    self.busy.is_none() && self.import.is_none(),
                    design::primary(if self.recorder.is_some() {
                        "Stop recording"
                    } else {
                        "Start recording"
                    })
                    .min_size(button_rect.size()),
                )
                .clicked()
            {
                self.toggle_recording();
            }
        });
        // A restrained waveform: animation indicates recording, not microphone amplitude.
        let wave_center = pos2(rect.right() - rect.width() * 0.20, rect.center().y);
        let wave_width = (rect.width() * 0.27).min(232.);
        let time = ui.input(|i| i.time) as f32;
        for i in 0..29 {
            let x = i as f32 / 28.;
            let envelope = (1. - (x * 2. - 1.).abs()).powf(0.6);
            let motion = if self.recorder.is_some() {
                0.55 + 0.45 * (time * 4. + i as f32 * 0.9).sin().abs()
            } else {
                0.35 + 0.65 * (i as f32 * 1.8).sin().abs()
            };
            let height = 8. + envelope * 79. * motion;
            let bar = Rect::from_center_size(
                wave_center + vec2((x - 0.5) * wave_width, 0.),
                vec2(3.5, height),
            );
            ui.painter().rect_filled(
                bar,
                2,
                p.accent.gamma_multiply(if p.dark {
                    0.55 + envelope * 0.45
                } else {
                    0.4 + envelope * 0.4
                }),
            );
        }
        ui.add_space(18.);
        let words: usize = self
            .store
            .entries
            .iter()
            .map(|e| e.text.split_whitespace().count())
            .sum();
        let seconds = self.store.entries.iter().fold(0.0, |n, e| n + e.seconds);
        let stats = [
            (self.store.entries.len().to_string(), "Recordings"),
            (words.to_string(), "Words"),
            (format!("{:.1}", seconds / 60.), "Minutes"),
        ];
        let (stats_rect, stats_response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 76.), egui::Sense::hover());
        stats_response.on_hover_text("Totals for recordings currently in History");
        for (i, (value, label)) in stats.iter().enumerate() {
            let x = stats_rect.left() + i as f32 * stats_rect.width() / 3.;
            if i > 0 {
                ui.painter().line_segment(
                    [
                        pos2(x, stats_rect.top() + 16.),
                        pos2(x, stats_rect.bottom() - 14.),
                    ],
                    Stroke::new(1.0_f32, p.border),
                );
            }
            ui.painter().text(
                pos2(x + 20., stats_rect.top() + 8.),
                Align2::LEFT_TOP,
                value,
                FontId::proportional(25.),
                p.ink,
            );
            ui.painter().text(
                pos2(x + 20., stats_rect.top() + 44.),
                Align2::LEFT_TOP,
                label,
                FontId::proportional(12.),
                p.muted,
            );
        }
        ui.add_space(18.);
        self.editor(ui);
    }
    fn editor(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        design::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Transcript").size(16.).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !self.text.trim().is_empty(),
                            egui::Button::new("Read aloud").frame(false),
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
                    if design::icon_button(ui, Icon::Stop, "Stop reading", false).clicked() {
                        let _ = self.speech.send.send(Command::Stop);
                    }
                });
            });
            ui.add_space(8.);
            ui.add_sized(
                [ui.available_width(), 135.],
                egui::TextEdit::multiline(&mut self.text)
                    .frame(false)
                    .font(FontId::proportional(16.))
                    .hint_text("Your transcript will appear here."),
            );
            ui.add_space(6.);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} words", self.text.split_whitespace().count()))
                        .small()
                        .color(p.muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(!self.text.is_empty(), design::secondary("Export text"))
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
                    if self.selected.is_some()
                        && ui
                            .add_enabled(
                                !self.locked(),
                                egui::Button::new("Save edits").frame(false),
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
            });
        });
    }
    fn history(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        ui.horizontal(|ui| {
            Self::title(ui, "History");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} / 5 recordings", self.store.entries.len()))
                        .color(p.muted)
                        .size(13.),
                );
            });
        });
        ui.add_space(20.);
        if self.store.entries.is_empty() {
            design::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add_space(40.);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No recordings yet").size(20.));
                    ui.label(
                        RichText::new("Your last five recordings will appear here.").color(p.muted),
                    );
                });
                ui.add_space(40.);
            });
        }
        let (mut select, mut retry, mut export, mut delete) = (None, None, None, None);
        for entry in self.store.entries.clone() {
            design::card(p).inner_margin(18.).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new(&entry.label).strong().size(15.))
                                .frame(false),
                        )
                        .clicked()
                    {
                        select = Some(entry.id);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        design::badge(
                            ui,
                            match entry.status {
                                Status::Ready => "Complete",
                                Status::Failed => "Needs retry",
                                Status::Recording => "Recording",
                                Status::Transcribing => "Transcribing",
                            },
                            if entry.status == Status::Failed {
                                p.danger
                            } else {
                                p.accent
                            },
                        );
                    });
                });
                ui.label(
                    RichText::new(format!(
                        "{}   ·   {:.0} sec",
                        entry
                            .created
                            .with_timezone(&chrono::Local)
                            .format("%d %b, %H:%M"),
                        entry.seconds
                    ))
                    .size(12.)
                    .color(p.muted),
                );
                if !entry.error.is_empty() {
                    ui.label(
                        RichText::new("Transcription failed. Hover for details.")
                            .color(p.danger)
                            .size(13.),
                    )
                    .on_hover_text(&entry.error);
                }
                if !entry.text.is_empty() {
                    ui.label(
                        RichText::new(entry.text.chars().take(145).collect::<String>())
                            .color(p.muted),
                    );
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.locked(), design::secondary("Retry"))
                        .clicked()
                    {
                        retry = Some(entry.id);
                    }
                    if ui
                        .add_enabled(
                            self.recorder.as_ref().map(|r| r.0) != Some(entry.id),
                            egui::Button::new("Export audio").frame(false),
                        )
                        .clicked()
                    {
                        export = Some(entry.id);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled_ui(!self.locked(), |ui| {
                            if design::icon_button(ui, Icon::Trash, "Delete recording", false)
                                .clicked()
                            {
                                self.delete_confirm = Some(entry.id);
                            }
                        });
                    });
                });
                if self.delete_confirm == Some(entry.id) {
                    ui.horizontal(|ui| {
                        ui.label("Delete audio and text?");
                        if ui.button("Delete").clicked() {
                            delete = Some(entry.id);
                        }
                        if ui.button("Cancel").clicked() {
                            self.delete_confirm = None;
                        }
                    });
                }
            });
            ui.add_space(4.);
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
        if self.selected.is_some() {
            ui.add_space(12.);
            self.editor(ui);
        }
    }
    fn dictionary(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        ui.horizontal(|ui| {
            Self::title(ui, "Dictionary");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(design::primary("Save")).clicked() {
                    let result = self.store.save_settings();
                    if result.is_ok() {
                        self.message = "Dictionary saved.".into();
                    }
                    self.report(result);
                }
                if ui.add(design::secondary("Add correction")).clicked() {
                    self.store.settings.rules.push(Rule::default());
                }
            });
        });
        ui.label(RichText::new("Correct names and frequently misheard words.").color(p.muted));
        ui.add_space(20.);
        design::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if self.store.settings.rules.is_empty() {
                ui.add_space(35.);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No corrections yet").size(20.));
                });
                ui.add_space(35.);
            }
            let mut remove = None;
            let width = (ui.available_width() - 70.) / 2.;
            if !self.store.settings.rules.is_empty() {
                ui.horizontal(|ui| {
                    for label in ["REPLACE", "WITH"] {
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(width, 18.), egui::Sense::hover());
                        ui.painter().text(
                            rect.left_center(),
                            Align2::LEFT_CENTER,
                            label,
                            FontId::proportional(11.),
                            p.muted,
                        );
                    }
                });
            }
            for (i, rule) in self.store.settings.rules.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [width, 38.],
                        egui::TextEdit::singleline(&mut rule.heard)
                            .margin(vec2(10., 9.))
                            .hint_text("Misheard word"),
                    );
                    ui.add_sized(
                        [width, 38.],
                        egui::TextEdit::singleline(&mut rule.replacement)
                            .margin(vec2(10., 9.))
                            .hint_text("Correct spelling"),
                    );
                    if design::icon_button(ui, Icon::Trash, "Remove correction", false).clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                self.store.settings.rules.remove(i);
            }
        });
    }
    fn settings(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        ui.horizontal(|ui| {
            Self::title(ui, "Settings");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(design::primary("Save changes")).clicked() {
                    let result = self.store.save_settings();
                    if result.is_ok() {
                        self.message = "Settings saved.".into();
                    }
                    self.report(result);
                }
            });
        });
        ui.add_space(20.);
        ui.horizontal(|ui| {
            use crate::store::Language;
            ui.label("Indicator language / Sprache");
            egui::ComboBox::from_id_salt("language")
                .selected_text(self.store.settings.language.text("English", "Deutsch"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.store.settings.language,
                        Language::English,
                        "English",
                    );
                    ui.selectable_value(
                        &mut self.store.settings.language,
                        Language::German,
                        "Deutsch",
                    );
                });
        });
        ui.add_space(12.);
        design::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Appearance").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.theme_switch(ui);
                    ui.label(
                        RichText::new(match self.store.settings.theme {
                            Theme::System => "Follow Windows",
                            Theme::Light => "Light",
                            Theme::Dark => "Dark",
                        })
                        .color(p.muted),
                    );
                });
            });
        });
        ui.add_space(8.);
        design::card(p).show(ui,|ui|{
            ui.set_width(ui.available_width());ui.label(RichText::new("Recording").size(16.).strong());ui.add_space(12.);
            ui.horizontal(|ui|{ui.add_sized([120.,38.],egui::Label::new("Microphone"));egui::ComboBox::from_id_salt("microphone").selected_text(if self.store.settings.microphone.is_empty(){"Windows default"}else{&self.store.settings.microphone}).width((ui.available_width()-60.).max(180.)).show_ui(ui,|ui|{ui.selectable_value(&mut self.store.settings.microphone,String::new(),"Windows default");for device in &self.devices{ui.selectable_value(&mut self.store.settings.microphone,device.clone(),device);}});if ui.add(egui::Button::new("Refresh").frame(false)).clicked(){self.devices=audio::microphones();}});
            ui.add_space(4.);
            ui.horizontal(|ui|{ui.add_sized([120.,38.],egui::Label::new("Shortcut"));ui.add(egui::TextEdit::singleline(&mut self.hotkey_edit).margin(vec2(10., 9.)).desired_width((ui.available_width()-100.).max(150.))).on_hover_text("For example: Ctrl+Shift+Space or Ctrl+Alt+R");if ui.add(design::secondary("Apply")).clicked(){match self.register_hotkey(){Ok(())=>self.message="Shortcut saved.".into(),Err(e)=>self.message=format!("Shortcut unavailable: {e}")}}});
            ui.add_space(6.);ui.separator();ui.add_space(6.);
            ui.checkbox(&mut self.store.settings.type_into_app,"Type into the active app").on_hover_text("Sends text to the app where recording started. Skipped if the destination window changes. Off keeps text in Whispering.");
        });
        ui.add_space(8.);
        design::card(p).show(ui,|ui|{
            ui.set_width(ui.available_width());ui.horizontal(|ui|{ui.vertical(|ui|{ui.label(RichText::new("Recording recovery").strong());ui.label(RichText::new("Keep the latest 5 recordings").size(13.).color(p.muted));});ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                egui::ComboBox::from_id_salt("retention").selected_text(match self.store.settings.retention_hours{1=>"1 hour",168=>"7 days",_=>"24 hours"}).width(135.).show_ui(ui,|ui|{for(hours,label)in [(1,"1 hour"),(24,"24 hours"),(168,"7 days")]{ui.selectable_value(&mut self.store.settings.retention_hours,hours,label);}}).response.on_hover_text("The sixth recording replaces the oldest, including failures. Expired audio and text are deleted while open or on the next launch.");
            });});
        });
        ui.add_space(12.);
        egui::CollapsingHeader::new("Storage & privacy").show(ui,|ui|{ui.label("Parakeet v3 and Windows read-aloud run locally. Exports and automatic typing go to the destination you choose.");ui.label(format!("Data: {}",self.store.root.display()));ui.label("Local files are not encrypted by Whispering. Full details are in README.md.");});
    }
    fn guide(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        Self::title(ui, "Getting started");
        ui.add_space(22.);
        for (number, title, detail) in [
            (
                "01",
                "Choose your microphone",
                "Use the Windows default or select one in Settings.",
            ),
            (
                "02",
                "Press. Speak. Press again.",
                &format!("{} starts and stops recording.", self.store.settings.hotkey),
            ),
            (
                "03",
                "Retry from History",
                "Your last 5 recordings are saved for 24 hours by default.",
            ),
        ] {
            design::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(number).size(25.).color(p.accent));
                    ui.add_space(12.);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(title).size(18.).strong());
                        ui.label(RichText::new(detail).color(p.muted));
                    });
                });
            });
            ui.add_space(8.);
        }
        ui.add_space(12.);
        if ui.add(design::primary("Start dictating")).clicked() {
            self.store.settings.onboarded = true;
            self.report(self.store.save_settings());
            self.page = Page::Dashboard;
        }
        ui.add_space(12.);
        egui::CollapsingHeader::new("Microphone not working?").show(ui,|ui|{ui.label("Enable desktop microphone access in Windows Settings > Privacy & security > Microphone.");});
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.; 4]
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        self.floating_indicator(ctx);
        ctx.request_repaint_after(Duration::from_millis(if self.recorder.is_some() {
            33
        } else {
            100
        }));
        if self.floating_job.is_none() {
            self.capture_preview(ctx);
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.recorder.is_some() {
            self.toggle_recording();
        }
        let p = Palette::new(ctx.theme() == egui::Theme::Dark);
        egui::SidePanel::left("navigation")
            .exact_width(216.)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.sidebar).inner_margin(18.))
            .show(ctx, |ui| {
                ui.add_space(12.);
                ui.horizontal(|ui| {
                    ui.image((self.logo.id(), vec2(32., 32.)));
                    ui.label(RichText::new("Whispering").size(19.).strong());
                });
                ui.add_space(38.);
                for (page, symbol, label) in [
                    (Page::Dashboard, Icon::Mic, "Dictation"),
                    (Page::History, Icon::History, "History"),
                    (Page::Dictionary, Icon::Book, "Dictionary"),
                ] {
                    if design::nav(ui, symbol, label, self.page == page).clicked() {
                        self.page = page;
                    }
                }
                ui.add_space((ui.available_height() - 290.).max(16.));
                if design::nav(ui, Icon::Settings, "Settings", self.page == Page::Settings)
                    .clicked()
                {
                    self.page = Page::Settings;
                }
                if design::nav(ui, Icon::Help, "Quick start", self.page == Page::Guide).clicked() {
                    self.page = Page::Guide;
                }
                ui.add_space(12.);
                self.theme_switch(ui);
                ui.add_space(12.);
                egui::Frame::new()
                    .fill(p.surface)
                    .corner_radius(10)
                    .inner_margin(13.)
                    .show(ui, |ui| {
                        ui.set_width(152.);
                        ui.horizontal(|ui| {
                            let (dot, _) =
                                ui.allocate_exact_size(vec2(8., 8.), egui::Sense::hover());
                            ui.painter().circle_filled(
                                dot.center(),
                                3.,
                                if self.model_ready { p.accent } else { p.muted },
                            );
                            ui.label(RichText::new("Parakeet v3").size(13.).strong());
                        });
                        ui.label(
                            RichText::new(if self.model_ready {
                                "On device · Ready"
                            } else if self.model == "Model unavailable" {
                                "Model unavailable"
                            } else {
                                "Loading model…"
                            })
                            .size(11.)
                            .color(p.muted),
                        )
                        .on_hover_text(&self.model);
                    });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(32.))
            .show(ctx, |ui| {
                if !self.message.is_empty() {
                    egui::Frame::new()
                        .fill(p.raised)
                        .corner_radius(10)
                        .inner_margin(12.)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(&self.message);
                                if ui.small_button("Dismiss").clicked() {
                                    self.message.clear();
                                }
                            });
                        });
                    ui.add_space(8.);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
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

impl App {
    pub(super) fn capture_preview(&mut self, ctx: &egui::Context) {
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
                    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
                }
            }
        }
    }
}
