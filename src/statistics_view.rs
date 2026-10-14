use super::*;
use crate::{
    statistics::{Period, Summary},
    ui::{self as design, Palette},
};
use egui::{vec2, Rect};

impl App {
    fn period_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (period, label) in [
                (Period::Week, "Weekly"),
                (Period::Month, "Monthly"),
                (Period::Total, "Total"),
            ] {
                ui.selectable_value(&mut self.stats_period, period, label)
                    .on_hover_text(period.description());
            }
        });
    }
    fn summary(&self) -> Summary {
        Summary::new(
            &self.store.statistics,
            self.stats_period,
            chrono::Local::now().date_naive(),
        )
    }
    pub(super) fn dashboard_statistics(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        ui.horizontal(|ui| {
            self.period_picker(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("View statistics").clicked() {
                    self.page = Page::Statistics;
                }
            });
        });
        let summary = self.summary();
        ui.add_space(6.);
        ui.columns(3, |columns| {
            for (ui, (value, label)) in columns.iter_mut().zip([
                (summary.words.to_string(), "Spoken words"),
                (format!("{:.0}", summary.wpm()), "Words / minute"),
                (
                    format!("{:.1} min", summary.saved_minutes()),
                    "Time saved · estimated",
                ),
            ]) {
                ui.label(RichText::new(value).size(25.));
                ui.label(RichText::new(label).size(12.).color(p.muted));
            }
        });
    }
    pub(super) fn statistics_page(&mut self, ui: &mut egui::Ui) {
        let p = Palette::of(ui);
        ui.heading(RichText::new("Statistics").size(29.).strong());
        ui.label(RichText::new("Your dictation activity, saved on this device.").color(p.muted));
        ui.add_space(16.);
        self.period_picker(ui);
        ui.label(
            RichText::new(self.stats_period.description())
                .small()
                .color(p.muted),
        );
        ui.add_space(14.);
        let summary = self.summary();
        let metrics = [
            (summary.dictations.to_string(), "Dictations"),
            (summary.words.to_string(), "Spoken words"),
            (format!("{:.0}", summary.wpm()), "Words / minute"),
            (summary.active_days.to_string(), "Active days"),
            (format!("{} days", summary.current_streak), "Current streak"),
            (format!("{} days", summary.longest_streak), "Longest streak"),
            (summary.apps.len().to_string(), "Apps used"),
            (
                format!("{:.1} min", summary.saved_minutes()),
                "Time saved · estimated",
            ),
            (
                format!("{:.1} min", summary.seconds / 60.),
                "Time dictating",
            ),
        ];
        for row in metrics.chunks(3) {
            ui.columns(3, |columns| {
                for (ui, (value, label)) in columns.iter_mut().zip(row) {
                    design::card(p).inner_margin(16.).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(value).size(24.));
                        ui.label(RichText::new(*label).size(12.).color(p.muted));
                    });
                }
            });
            ui.add_space(4.);
        }
        ui.add_space(10.);
        design::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Daily activity").strong());
            ui.label(RichText::new(if self.stats_period == Period::Total {
                "Spoken words · most recent 30 days"
            } else { "Spoken words in the selected period" }).small().color(p.muted));
            let today = chrono::Local::now().date_naive();
            let count = if self.stats_period == Period::Week { 7 } else { 30 };
            let start = today - chrono::Days::new(count - 1);
            let daily = Summary::new(&self.store.statistics, if count == 7 { Period::Week } else { Period::Month }, today);
            let max = daily.days.values().copied().max().unwrap_or(1).max(1) as f32;
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 84.), egui::Sense::hover());
            let stride = rect.width() / count as f32;
            for i in 0..count {
                let day = start + chrono::Days::new(i);
                let words = daily.days.get(&day).copied().unwrap_or(0);
                let height = (words as f32 / max * 76.).max(2.);
                let bar = Rect::from_min_max(
                    egui::pos2(rect.left() + i as f32 * stride + 2., rect.bottom() - height),
                    egui::pos2(rect.left() + (i + 1) as f32 * stride - 2., rect.bottom()),
                );
                ui.painter().rect_filled(bar, 3, if words > 0 { p.accent } else { p.border });
                ui.interact(Rect::from_min_max(egui::pos2(bar.left(), rect.top()), bar.max), ui.id().with(("activity", i)), egui::Sense::hover())
                    .on_hover_text(format!("{}: {} words", day.format("%d %b"), words));
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new(start.format("%d %b").to_string()).small().color(p.muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(RichText::new("Today").small().color(p.muted)); });
            });
            if summary.dictations == 0 { ui.label("No dictations in this period yet. Your first completed recording will appear here."); }
        });
        ui.add_space(10.);
        design::card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Apps used").strong());
            let mut apps: Vec<_> = summary.apps.iter().collect();
            apps.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            if apps.is_empty() {
                ui.label(RichText::new("No app usage recorded in this period.").color(p.muted));
            }
            for (name, count) in apps {
                ui.horizontal(|ui| {
                    ui.label(name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{count} dictations")).color(p.muted));
                    });
                });
            }
        });
        ui.add_space(10.);
        ui.label(RichText::new("Time saved compares dictation with typing at 40 words/minute. Speed uses total words divided by recording time. Streaks count consecutive local calendar days within the selected period; today's streak stays active if you dictated yesterday.").small().color(p.muted));
        ui.label(RichText::new("Statistics survive recording deletion. Imports are excluded. Existing completed recordings seed the totals; older deleted recordings and their app usage cannot be recovered.").small().color(p.muted));
    }
}
