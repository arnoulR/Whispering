use super::*;
use egui::{pos2, vec2, Align2, FontId, Rect, Sense, Stroke};

const SIZE: egui::Vec2 = vec2(260., 60.);

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
        let seconds = self
            .recorder
            .as_ref()
            .map(|(_, r)| r.started.elapsed().as_secs())
            .unwrap_or(if preview_recording { 83 } else { 0 })
            .min(audio::MAX_SECONDS);
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
        let mut moved = None;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("recording-indicator"),
            egui::ViewportBuilder::default()
                .with_title("Whispering recording indicator")
                .with_inner_size(SIZE)
                .with_position(position)
                .with_decorations(false)
                .with_transparent(true)
                .with_mouse_passthrough(false)
                .with_resizable(false)
                .with_taskbar(false)
                .with_active(false)
                .with_always_on_top(),
            |ctx, _| {
                configure_indicator(ctx.pixels_per_point(), &mut self.floating_native);
                moved = ctx.input(|i| i.viewport().outer_rect.map(|r| r.min));
                let p = crate::ui::Palette::new(ctx.theme() == egui::Theme::Dark);
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        // Leave transparent space around the antialiased outline instead of
                        // clipping it to a binary Win32 region (which produces jagged edges).
                        let bounds = ui.max_rect();
                        let body = bounds.shrink(2.);
                        ui.painter().rect(
                            body,
                            12,
                            p.surface,
                            Stroke::new(1.0_f32, Color32::BLACK),
                            egui::StrokeKind::Inside,
                        );
                        let drag = ui.interact(body, ui.id().with("move-indicator"), Sense::drag());
                        let origin = bounds.min;
                        if recording {
                            ui.painter().text(
                                origin + vec2(16., 20.),
                                Align2::LEFT_CENTER,
                                language.text("Recording", "Aufnahme"),
                                FontId::proportional(14.),
                                p.ink,
                            );
                            ui.painter().text(
                                origin + vec2(16., 40.),
                                Align2::LEFT_CENTER,
                                elapsed_label(seconds),
                                FontId::proportional(12.),
                                p.muted,
                            );
                            for (i, amplitude) in self.waveform.iter().enumerate() {
                                let center = origin + vec2(126. + i as f32 * 4.8, 30.);
                                ui.painter().rect_filled(
                                    Rect::from_center_size(center, vec2(2., 2. + amplitude * 20.)),
                                    1,
                                    p.accent,
                                );
                            }
                        } else if failed {
                            let label = language
                                .text("Transcription failed", "Transkription fehlgeschlagen");
                            let label = ui.painter().layout_no_wrap(
                                label.into(),
                                FontId::proportional(12.),
                                p.ink,
                            );
                            let label_width = label.size().x;
                            ui.painter().text(
                                origin + vec2(14., 30.),
                                Align2::LEFT_CENTER,
                                language
                                    .text("Transcription failed", "Transkription fehlgeschlagen"),
                                FontId::proportional(12.),
                                p.ink,
                            );
                            ui.interact(
                                Rect::from_min_size(
                                    origin + vec2(14., 19.),
                                    vec2(label_width, 22.),
                                ),
                                ui.id().with("error-detail"),
                                Sense::hover(),
                            )
                            .on_hover_text(&error);
                            let retry_rect = Rect::from_min_size(
                                origin + vec2(22. + label_width, 19.),
                                vec2(44., 22.),
                            );
                            retry = ui
                                .add_enabled_ui(!self.locked(), |ui| {
                                    ui.spacing_mut().button_padding = vec2(6., 3.);
                                    ui.spacing_mut().interact_size = vec2(0., 22.);
                                    ui.put(
                                        retry_rect,
                                        egui::Button::new(
                                            RichText::new(language.text("Retry", "Erneut"))
                                                .size(11.),
                                        )
                                        .corner_radius(6),
                                    )
                                })
                                .inner
                                .clicked();
                            let close_rect =
                                Rect::from_min_size(origin + vec2(232., 5.), vec2(20., 20.));
                            let close = ui.interact(
                                close_rect,
                                ui.id().with("close-indicator"),
                                Sense::click(),
                            );
                            let ink = if close.hovered() { p.ink } else { p.muted };
                            crate::ui::icon(ui, crate::ui::Icon::Close, close_rect.shrink(3.), ink);
                            dismiss = close.clicked();
                        } else {
                            ui.put(
                                Rect::from_min_size(origin + vec2(16., 21.), vec2(18., 18.)),
                                egui::Spinner::new().size(18.),
                            );
                            ui.painter().text(
                                origin + vec2(46., 30.),
                                Align2::LEFT_CENTER,
                                language.text("Transcribing…", "Transkription…"),
                                FontId::proportional(14.),
                                p.ink,
                            );
                        }
                        if drag.drag_started_by(egui::PointerButton::Primary) {
                            drag_indicator();
                        }
                    });
                dismiss |= ctx.input(|i| i.viewport().close_requested());
                self.capture_preview(ctx);
            },
        );
        if let Some(position) = moved {
            self.floating_position = Some(position);
        }
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

fn elapsed_label(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

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
            return pos2(
                (info.rcWork.left + info.rcWork.right) as f32 / (2. * scale) - SIZE.x / 2.,
                info.rcWork.bottom as f32 / scale - SIZE.y - 24.,
            );
        }
    }
    pos2(400., 400.)
}

// Accept clicks and native window dragging without stealing the text field's focus.
fn configure_indicator(scale: f32, applied: &mut Option<(usize, i32, i32, i32)>) {
    use windows::{
        core::w,
        Win32::{
            Foundation::RECT,
            Graphics::{Dwm::*, Gdi::*},
            UI::Controls::MARGINS,
            UI::WindowsAndMessaging::*,
        },
    };
    unsafe {
        let Ok(window) = FindWindowW(None, w!("Whispering recording indicator")) else {
            return;
        };
        let mut size = RECT::default();
        if GetClientRect(window, &mut size).is_err() {
            return;
        }
        let key = (
            window.0 as usize,
            size.right,
            size.bottom,
            (scale * 1000.).round() as i32,
        );
        if *applied == Some(key) {
            return;
        }
        let mut process = 0;
        GetWindowThreadProcessId(window, Some(&mut process));
        if process != std::process::id() {
            return;
        }
        let style = GetWindowLongPtrW(window, GWL_EXSTYLE);
        SetWindowLongPtrW(window, GWL_EXSTYLE, style | WS_EX_NOACTIVATE.0 as isize);
        // Extend composition across the OpenGL client area so transparent pixels
        // (including the antialiased rounded outline) blend with the desktop.
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        if let Err(error) = DwmExtendFrameIntoClientArea(window, &margins) {
            eprintln!("Indicator transparency: {error}");
        }
        // The GL surface can still have an opaque surround on some Windows
        // configurations. Remove it from the native window, including hit testing.
        let mut rect = RECT::default();
        if GetClientRect(window, &mut rect).is_ok() {
            let inset = (2. * scale).ceil() as i32;
            let diameter = (24. * scale).round() as i32;
            let region = CreateRoundRectRgn(
                inset,
                inset,
                rect.right - inset + 1,
                rect.bottom - inset + 1,
                diameter,
                diameter,
            );
            if !region.0.is_null() && SetWindowRgn(window, region, true) == 0 {
                let _ = DeleteObject(region);
            }
        }
        *applied = Some(key);
    }
}

// egui-winit refuses StartDrag for unfocused windows. This indicator intentionally
// uses NOACTIVATE, so start Windows' move loop directly while the button is down.
fn drag_indicator() {
    use windows::{
        core::w,
        Win32::{
            Foundation::{LPARAM, WPARAM},
            UI::{Input::KeyboardAndMouse::ReleaseCapture, WindowsAndMessaging::*},
        },
    };
    unsafe {
        let Ok(window) = FindWindowW(None, w!("Whispering recording indicator")) else {
            return;
        };
        let mut process = 0;
        GetWindowThreadProcessId(window, Some(&mut process));
        if process != std::process::id() {
            return;
        }
        let _ = ReleaseCapture();
        SendMessageW(
            window,
            WM_SYSCOMMAND,
            WPARAM((SC_MOVE | HTCAPTION) as usize),
            LPARAM(0),
        );
    }
}
