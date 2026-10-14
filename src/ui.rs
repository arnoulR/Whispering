use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, RichText, Stroke};

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub sidebar: Color32,
    pub surface: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub selection: Color32,
    pub danger: Color32,
}
impl Palette {
    pub fn new(dark: bool) -> Self {
        let c = Color32::from_rgb;
        if dark {
            Self {
                dark,
                bg: c(9, 13, 22),
                sidebar: c(12, 17, 28),
                surface: c(17, 24, 38),
                raised: c(24, 33, 50),
                border: c(34, 45, 64),
                ink: c(239, 243, 252),
                muted: c(145, 159, 183),
                accent: c(99, 146, 255),
                selection: c(26, 43, 73),
                danger: c(255, 148, 148),
            }
        } else {
            Self {
                dark,
                bg: c(244, 246, 251),
                sidebar: c(255, 255, 255),
                surface: c(255, 255, 255),
                raised: c(241, 244, 250),
                border: c(223, 229, 240),
                ink: c(20, 29, 48),
                muted: c(99, 114, 139),
                accent: c(47, 96, 215),
                selection: c(233, 240, 255),
                danger: c(181, 48, 65),
            }
        }
    }
    pub fn of(ui: &egui::Ui) -> Self {
        Self::new(ui.visuals().dark_mode)
    }
}
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Roboto".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Roboto.ttf")).into(),
    );
    // Cover all app text, including editors and shortcut labels; retain glyph fallbacks.
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .get_mut(&family)
            .unwrap()
            .insert(0, "Roboto".into());
    }
    egui_extras::install_image_loaders(ctx);
    ctx.set_fonts(fonts);
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let p = Palette::new(theme == egui::Theme::Dark);
        let mut style = theme.default_style();
        style.spacing.item_spacing = vec2(12., 10.);
        style.spacing.button_padding = vec2(16., 10.);
        style.spacing.interact_size = vec2(38., 38.);
        style.visuals.override_text_color = Some(p.ink);
        style.visuals.panel_fill = p.bg;
        style.visuals.window_fill = p.surface;
        style.visuals.extreme_bg_color = p.bg;
        style.visuals.faint_bg_color = p.raised;
        style.visuals.window_stroke = Stroke::new(1.0_f32, p.border);
        style.visuals.window_corner_radius = 12.into();
        style.visuals.selection.bg_fill = p.selection;
        style.visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);
        for w in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.noninteractive,
        ] {
            w.bg_fill = p.raised;
            w.weak_bg_fill = p.raised;
            w.bg_stroke = Stroke::new(1.0_f32, p.border);
            w.fg_stroke = Stroke::new(1.0_f32, p.ink);
            w.corner_radius = 8.into();
        }
        for w in [
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            w.bg_fill = p.selection;
            w.weak_bg_fill = p.selection;
            w.bg_stroke = Stroke::new(1.0_f32, p.accent);
            w.fg_stroke = Stroke::new(1.0_f32, p.ink);
            w.corner_radius = 8.into();
        }
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(12.));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(29.));
        ctx.set_style_of(theme, style);
    }
}
pub fn card(p: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(14)
        .inner_margin(22.)
}
pub fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(Color32::WHITE).size(14.))
        .fill(Color32::from_rgb(53, 104, 235))
        .stroke(Stroke::NONE)
        .corner_radius(9)
        .min_size(vec2(0., 40.))
}
pub fn secondary(text: &str) -> egui::Button<'_> {
    egui::Button::new(text)
        .corner_radius(9)
        .min_size(vec2(0., 38.))
}

#[derive(Clone, Copy)]
pub enum Icon {
    Mic,
    History,
    Book,
    Settings,
    Help,
    Sun,
    Moon,
    Monitor,
    Trash,
    Play,
    Pause,
    Statistics,
    Close,
}
pub fn icon(ui: &egui::Ui, icon: Icon, rect: Rect, color: Color32) {
    let source = match icon {
        Icon::Mic => egui::include_image!("../assets/icons/mic.svg"),
        Icon::History => egui::include_image!("../assets/icons/history.svg"),
        Icon::Book => egui::include_image!("../assets/icons/book-open.svg"),
        Icon::Settings => egui::include_image!("../assets/icons/settings-2.svg"),
        Icon::Help => egui::include_image!("../assets/icons/circle-help.svg"),
        Icon::Sun => egui::include_image!("../assets/icons/sun.svg"),
        Icon::Moon => egui::include_image!("../assets/icons/moon.svg"),
        Icon::Monitor => egui::include_image!("../assets/icons/monitor.svg"),
        Icon::Trash => egui::include_image!("../assets/icons/trash-2.svg"),
        Icon::Play => egui::include_image!("../assets/icons/play.svg"),
        Icon::Pause => egui::include_image!("../assets/icons/pause.svg"),
        Icon::Statistics => egui::include_image!("../assets/icons/chart-no-axes-combined.svg"),
        Icon::Close => egui::include_image!("../assets/icons/x.svg"),
    };
    egui::Image::new(source).tint(color).paint_at(ui, rect);
}
pub fn nav(ui: &mut egui::Ui, symbol: Icon, label: &str, selected: bool) -> egui::Response {
    let p = Palette::of(ui);
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 44.), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    let color = if selected {
        p.accent
    } else if response.hovered() {
        p.ink
    } else {
        p.muted
    };
    if selected || response.hovered() {
        ui.painter()
            .rect_filled(rect, 9, if selected { p.selection } else { p.raised });
    }
    if selected {
        ui.painter().rect_filled(
            Rect::from_min_size(rect.left_center() + vec2(0., -9.), vec2(3., 18.)),
            2,
            p.accent,
        );
    }
    icon(
        ui,
        symbol,
        Rect::from_center_size(rect.left_center() + vec2(24., 0.), vec2(19., 19.)),
        color,
    );
    ui.painter().text(
        rect.left_center() + vec2(45., 0.),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.),
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
pub fn icon_button(ui: &mut egui::Ui, symbol: Icon, label: &str, selected: bool) -> egui::Response {
    let p = Palette::of(ui);
    let response = ui.add_sized(
        [38., 34.],
        egui::Button::new("")
            .fill(if selected {
                p.selection
            } else {
                Color32::TRANSPARENT
            })
            .stroke(Stroke::NONE),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    icon(
        ui,
        symbol,
        Rect::from_center_size(response.rect.center(), vec2(18., 18.)),
        if selected { p.accent } else { p.muted },
    );
    response.on_hover_text(label)
}
pub fn badge(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.13))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(9, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(12.).color(color));
        });
}

pub fn gradient(painter: &egui::Painter, rect: Rect, radius: f32, left: Color32, right: Color32) {
    let color = |x: f32| {
        let t = ((x - rect.left()) / rect.width()).clamp(0., 1.);
        let a = left.to_array();
        let b = right.to_array();
        Color32::from_rgba_unmultiplied(
            (a[0] as f32 * (1. - t) + b[0] as f32 * t) as u8,
            (a[1] as f32 * (1. - t) + b[1] as f32 * t) as u8,
            (a[2] as f32 * (1. - t) + b[2] as f32 * t) as u8,
            255,
        )
    };
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.center(), color(rect.center().x));
    for (center, start) in [
        (pos2(rect.right() - radius, rect.top() + radius), -90.),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.),
        (pos2(rect.left() + radius, rect.bottom() - radius), 90.),
        (pos2(rect.left() + radius, rect.top() + radius), 180.),
    ] {
        for i in 0..=8 {
            let a = (start + i as f32 * 90. / 8.).to_radians();
            let p = center + vec2(a.cos(), a.sin()) * radius;
            mesh.colored_vertex(p, color(p.x));
        }
    }
    let n = mesh.vertices.len() as u32;
    for i in 1..n {
        mesh.add_triangle(0, i, if i + 1 == n { 1 } else { i + 1 });
    }
    painter.add(egui::Shape::mesh(mesh));
}
