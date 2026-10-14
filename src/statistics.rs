use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

// Deliberately contains no transcript, audio, window title, or full process path.
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: Uuid,
    pub day: NaiveDate,
    pub words: usize,
    pub seconds: f64,
    pub app: Option<String>,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Period {
    Week,
    Month,
    #[default]
    Total,
}
impl Period {
    pub fn start(self, today: NaiveDate) -> Option<NaiveDate> {
        match self {
            Self::Week => today.checked_sub_days(Days::new(6)),
            Self::Month => today.checked_sub_days(Days::new(29)),
            Self::Total => None,
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Week => "Last 7 days, including today",
            Self::Month => "Last 30 days, including today",
            Self::Total => "All recorded activity",
        }
    }
}

#[derive(Default)]
pub struct Summary {
    pub dictations: usize,
    pub words: usize,
    pub seconds: f64,
    pub active_days: usize,
    pub current_streak: usize,
    pub longest_streak: usize,
    pub apps: BTreeMap<String, usize>,
    pub days: BTreeMap<NaiveDate, usize>,
}
impl Summary {
    pub fn new(records: &[Record], period: Period, today: NaiveDate) -> Self {
        let mut result = Self::default();
        let start = period.start(today);
        for record in records
            .iter()
            .filter(|r| r.day <= today && start.is_none_or(|s| r.day >= s))
        {
            result.dictations += 1;
            result.words += record.words;
            result.seconds += record.seconds.max(0.);
            *result.days.entry(record.day).or_default() += record.words;
            if let Some(app) = &record.app {
                *result.apps.entry(app.clone()).or_default() += 1;
            }
        }
        let days: BTreeSet<_> = result.days.keys().copied().collect();
        result.active_days = days.len();
        let mut previous: Option<NaiveDate> = None;
        let mut streak = 0;
        for day in &days {
            streak = if previous.is_some_and(|p| p.succ_opt() == Some(*day)) {
                streak + 1
            } else {
                1
            };
            result.longest_streak = result.longest_streak.max(streak);
            previous = Some(*day);
        }
        let mut day = if days.contains(&today) {
            Some(today)
        } else {
            today.pred_opt()
        };
        while let Some(d) = day.filter(|d| days.contains(d)) {
            result.current_streak += 1;
            day = d.pred_opt();
        }
        result
    }
    pub fn wpm(&self) -> f64 {
        if self.seconds > 0. {
            self.words as f64 * 60. / self.seconds
        } else {
            0.
        }
    }
    pub fn saved_minutes(&self) -> f64 {
        (self.words as f64 / 40. - self.seconds / 60.).max(0.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(day: NaiveDate, words: usize, seconds: f64) -> Record {
        Record {
            id: Uuid::new_v4(),
            day,
            words,
            seconds,
            app: Some("notepad.exe".into()),
        }
    }
    #[test]
    fn periods_weighted_speed_and_streaks() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let records = vec![
            record(today, 120, 60.),
            record(today, 60, 60.),
            record(today - Days::new(1), 120, 60.),
            record(today - Days::new(6), 100, 60.),
            record(today - Days::new(7), 100, 60.),
            record(today - Days::new(29), 100, 60.),
            record(today - Days::new(30), 100, 60.),
        ];
        let week = Summary::new(&records, Period::Week, today);
        assert_eq!((week.dictations, week.words, week.active_days), (4, 400, 3));
        assert_eq!((week.current_streak, week.longest_streak), (2, 2));
        assert_eq!(week.wpm(), 100.);
        assert_eq!(week.saved_minutes(), 6.);
        assert_eq!(week.apps["notepad.exe"], 4);
        assert_eq!(Summary::new(&records, Period::Month, today).dictations, 6);
        assert_eq!(Summary::new(&records, Period::Total, today).dictations, 7);
    }
    #[test]
    fn yesterday_keeps_streak_alive_and_empty_data_is_zero() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let records = vec![
            record(today - Days::new(1), 5, 60.),
            record(today - Days::new(2), 5, 60.),
        ];
        assert_eq!(
            Summary::new(&records, Period::Total, today).current_streak,
            2
        );
        assert_eq!(
            Summary::new(&records, Period::Total, today + Days::new(1)).current_streak,
            0
        );
        assert_eq!(
            Summary::new(&records, Period::Total, today).saved_minutes(),
            0.
        );
        assert_eq!(Summary::new(&[], Period::Total, today).wpm(), 0.);
    }
}
