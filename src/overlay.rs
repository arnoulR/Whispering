use super::*;

impl App {
    pub(super) fn floating_indicator(&mut self, ctx: &egui::Context) {
        let Some(id) = self.floating_job else { return };
        let Some(entry) = self.store.entries.iter().find(|e| e.id == id) else {
            self.floating_job = None;
            return;
        };
        if entry.status == Status::Ready {
            self.floating_job = None;
            return;
        }
        let preview_recording = self.preview.as_ref().is_some_and(|(path, _, _)| {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .starts_with("overlay-recording")
        });
        let recording = preview_recording
            || self
                .recorder
                .as_ref()
                .is_some_and(|(current, _)| *current == id);
        let failed = entry.status == Status::Failed;
        let error = entry.error.clone();
        let language = self.store.settings.language;
        if recording && self.waveform_tick.elapsed() >= Duration::from_millis(33) {
            let level = self.recorder.as_ref().map(|(_, r)| r.level()).unwrap_or(0.);
            self.waveform.pop_front();
            self.waveform.push_back((level * 5.).sqrt().min(1.));
            self.waveform_tick = Instant::now();
        }
        if preview_recording {
            self.waveform = (0..24)
                .map(|i| (i as f32 * 0.8).sin().abs() * 0.8 + 0.1)
                .collect();
        }
        let position = *self
            .floating_position
            .get_or_insert_with(|| indicator_position(ctx));
        let mut retry = false;
        let mut dismiss = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("recording-indicator"),
            egui::ViewportBuilder::default()
                .with_title("Whispering")
                .with_inner_size([400., 64.])
                .with_position(position)
                .with_decorations(false)
                .with_transparent(true)
                .with_mouse_passthrough(!failed)
                .with_resizable(false)
                .with_taskbar(false)
                .with_active(false)
                .with_always_on_top(),
            |ctx, _| {
                let p = crate::ui::Palette::new(ctx.theme() == egui::Theme::Dark);
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(p.surface)
                            .corner_radius(24)
                            .inner_margin(16.),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal_centered(|ui| {
                            if recording {
                                ui.add_space(40.);
                                ui.label(
                                    RichText::new(language.text("Recording", "Aufnahme")).strong(),
                                );
                                ui.add_space(10.);
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(168., 28.),
                                    egui::Sense::hover(),
                                );
                                for (i, amplitude) in self.waveform.iter().enumerate() {
                                    let x = rect.left() + i as f32 * 7. + 3.;
                                    let h = 2. + amplitude * 24.;
                                    ui.painter().rect_filled(
                                        egui::Rect::from_center_size(
                                            egui::pos2(x, rect.center().y),
                                            egui::vec2(3., h),
                                        ),
                                        2.,
                                        p.accent,
                                    );
                                }
                            } else if failed {
                                ui.label(
                                    language.text(
                                        "Transcription failed",
                                        "Transkription fehlgeschlagen",
                                    ),
                                )
                                .on_hover_text(&error);
                                retry = ui
                                    .add_enabled(
                                        !self.locked(),
                                        egui::Button::new(language.text("Retry", "Erneut")),
                                    )
                                    .clicked();
                                dismiss = ui
                                    .button("×")
                                    .on_hover_text(language.text("Dismiss", "Schließen"))
                                    .clicked();
                            } else {
                                ui.spinner();
                                ui.label(language.text("Transcribing…", "Transkription…"));
                            }
                        });
                    });
                dismiss |= ctx.input(|i| i.viewport().close_requested());
                self.capture_preview(ctx);
            },
        );
        if retry {
            if let Some(target) = self.output_target {
                crate::output::restore_target(target);
            }
            self.transcribe(id);
        }
        if dismiss {
            self.floating_job = None;
        }
    }
}

// Anchor above the taskbar on the monitor where dictation started, including
// monitors with negative desktop coordinates. Keep the position during retry.
fn indicator_position(ctx: &egui::Context) -> egui::Pos2 {
    use windows::Win32::{Graphics::Gdi::*, UI::WindowsAndMessaging::GetForegroundWindow};
    unsafe {
        let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let scale = ctx.pixels_per_point();
            return egui::pos2(
                (info.rcWork.left + info.rcWork.right) as f32 / (2. * scale) - 200.,
                info.rcWork.bottom as f32 / scale - 88.,
            );
        }
    }
    egui::pos2(400., 400.)
}
