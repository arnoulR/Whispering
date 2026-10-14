use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const KEEP: usize = 10;
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Status {
    Recording,
    Transcribing,
    Ready,
    Failed,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: Uuid,
    pub created: DateTime<Utc>,
    pub label: String,
    pub seconds: f64,
    pub status: Status,
    pub text: String,
    pub error: String,
    #[serde(default)]
    pub app: Option<String>,
    #[serde(default)]
    pub imported: bool,
}
impl Entry {
    pub fn new(label: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            created: Utc::now(),
            label,
            seconds: 0.0,
            status: Status::Recording,
            text: String::new(),
            error: String::new(),
            app: None,
            imported: false,
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Rule {
    pub heard: String,
    pub replacement: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub hotkey: String,
    pub microphone: String,
    pub retention_hours: u32,
    pub onboarded: bool,
    pub type_into_app: bool,
    pub theme: Theme,
    pub language: Language,
    pub rules: Vec<Rule>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+Space".into(),
            microphone: String::new(),
            retention_hours: 24,
            onboarded: false,
            type_into_app: true,
            theme: Theme::System,
            language: Language::English,
            rules: vec![],
        }
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    German,
}
impl Language {
    pub fn text(self, english: &'static str, german: &'static str) -> &'static str {
        match self {
            Self::English => english,
            Self::German => german,
        }
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}
impl Theme {
    pub fn preference(self) -> eframe::egui::ThemePreference {
        match self {
            Self::System => eframe::egui::ThemePreference::System,
            Self::Light => eframe::egui::ThemePreference::Light,
            Self::Dark => eframe::egui::ThemePreference::Dark,
        }
    }
}
pub struct Store {
    pub root: PathBuf,
    pub entries: Vec<Entry>,
    pub settings: Settings,
    pub statistics: Vec<crate::statistics::Record>,
    _lock: File,
}

pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let tmp = path.with_extension("pending");
    let mut file = File::create(&tmp)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    drop(file);
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::PCWSTR,
        Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        },
    };
    let from: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )?;
    }
    Ok(())
}
impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("recordings"))?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("instance.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock)
            .context("Whispering is already running. Restore its window from the taskbar.")?;
        let settings = if root.join("settings.json").exists() {
            serde_json::from_slice(&fs::read(root.join("settings.json"))?)
                .context("Settings are damaged; your recordings have not been removed.")?
        } else {
            Settings::default()
        };
        let entries: Vec<Entry> = if root.join("history.json").exists() {
            let raw: Vec<serde_json::Value> =
                serde_json::from_slice(&fs::read(root.join("history.json"))?)
                    .context("History is damaged; your recordings have not been removed.")?;
            raw.into_iter()
                .map(|value| {
                    let legacy = value.get("imported").is_none();
                    let mut entry: Entry = serde_json::from_value(value)?;
                    if legacy {
                        entry.imported = entry.label != "Microphone recording";
                    }
                    Ok(entry)
                })
                .collect::<Result<_>>()?
        } else {
            vec![]
        };
        let statistics = if root.join("statistics.json").exists() {
            serde_json::from_slice(&fs::read(root.join("statistics.json"))?)
                .context("Statistics are damaged; your recordings have not been removed.")?
        } else {
            vec![]
        };
        let mut store = Self {
            root,
            settings,
            entries,
            statistics,
            _lock: lock,
        };
        for entry in &mut store.entries {
            if matches!(entry.status, Status::Recording | Status::Transcribing) {
                entry.status = Status::Failed;
                entry.error =
                    "Interrupted last time. Your saved audio is available to retry or export."
                        .into();
            }
        }
        store.sync_statistics()?;
        store.persist()?;
        store.prune(None)?;
        Ok(store)
    }
    pub fn audio(&self, id: Uuid) -> PathBuf {
        self.root.join("recordings").join(format!("{id}.wav"))
    }
    pub fn persist(&self) -> Result<()> {
        atomic_json(&self.root.join("history.json"), &self.entries)
    }
    pub fn save_settings(&self) -> Result<()> {
        atomic_json(&self.root.join("settings.json"), &self.settings)
    }
    pub fn sync_statistics(&mut self) -> Result<()> {
        let known: std::collections::HashSet<_> = self.statistics.iter().map(|r| r.id).collect();
        let mut updated = self.statistics.clone();
        for entry in self
            .entries
            .iter()
            .filter(|e| e.status == Status::Ready && !e.imported && !known.contains(&e.id))
        {
            updated.push(crate::statistics::Record {
                id: entry.id,
                day: entry.created.with_timezone(&chrono::Local).date_naive(),
                words: entry.text.split_whitespace().count(),
                seconds: if entry.seconds.is_finite() {
                    entry.seconds.max(0.)
                } else {
                    0.
                },
                app: entry.app.clone(),
            });
        }
        if updated.len() != self.statistics.len() || !self.root.join("statistics.json").exists() {
            atomic_json(&self.root.join("statistics.json"), &updated)
                .context("Could not save local statistics. Recordings have been retained.")?;
            self.statistics = updated;
        }
        Ok(())
    }
    pub fn add(&mut self, entry: Entry) -> Result<()> {
        self.sync_statistics()?;
        let id = entry.id;
        self.entries.insert(0, entry);
        if let Err(error) = self.persist() {
            self.entries.remove(0);
            return Err(error);
        }
        self.prune(Some(id))
    }
    pub fn prune(&mut self, active: Option<Uuid>) -> Result<()> {
        self.sync_statistics()?;
        let cutoff = Utc::now()
            - chrono::Duration::hours(self.settings.retention_hours.clamp(1, 168) as i64);
        let remove: Vec<Uuid> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(i, e)| Some(e.id) != active && (*i >= KEEP || e.created < cutoff))
            .map(|(_, e)| e.id)
            .collect();
        for id in remove {
            self.delete(id)?;
        }
        Ok(())
    }
    pub fn delete(&mut self, id: Uuid) -> Result<()> {
        self.sync_statistics()?;
        let audio = self.audio(id);
        if audio.exists() {
            fs::remove_file(audio).context("Could not delete recording; it remains in history.")?;
        }
        self.entries.retain(|e| e.id != id);
        self.persist()
    }
}
pub fn apply_rules(text: &str, rules: &[Rule]) -> String {
    let mut output = text.to_owned();
    for rule in rules.iter().filter(|r| !r.heard.trim().is_empty()) {
        let pattern = format!(r"(?i){}", regex::escape(rule.heard.trim()));
        if let Ok(re) = regex::Regex::new(&pattern) {
            let word = |c: char| c.is_alphanumeric() || c == '_';
            let mut replaced = String::new();
            let mut last = 0;
            for m in re.find_iter(&output) {
                if output[..m.start()].chars().next_back().is_some_and(word)
                    || output[m.end()..].chars().next().is_some_and(word)
                {
                    continue;
                }
                replaced.push_str(&output[last..m.start()]);
                replaced.push_str(&rule.replacement);
                last = m.end();
            }
            replaced.push_str(&output[last..]);
            output = replaced;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn statistics_survive_deletion_restart_and_do_not_double_count() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().into()).unwrap();
        let mut entry = Entry::new("Microphone recording".into());
        entry.status = Status::Ready;
        entry.text = "one two three".into();
        entry.seconds = 2.;
        let id = entry.id;
        store.add(entry).unwrap();
        store.sync_statistics().unwrap();
        store.entries[0].text = "edited transcript with many more words".into();
        store.sync_statistics().unwrap();
        assert_eq!(store.statistics.len(), 1);
        assert_eq!(store.statistics[0].words, 3);
        store.delete(id).unwrap();
        drop(store);
        let store = Store::open(dir.path().into()).unwrap();
        assert!(store.entries.is_empty());
        assert_eq!(store.statistics.len(), 1);
        assert_eq!(store.statistics[0].words, 3);
    }
    #[test]
    fn imports_and_failures_are_excluded_and_expiry_keeps_statistics() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().into()).unwrap();
        let mut import = Entry::new("file.wav".into());
        import.imported = true;
        import.status = Status::Ready;
        import.text = "imported words".into();
        store.add(import).unwrap();
        store.add(Entry::new("Failed".into())).unwrap();
        let mut old = Entry::new("Microphone recording".into());
        old.created = Utc::now() - chrono::Duration::days(2);
        old.status = Status::Ready;
        old.text = "saved statistic".into();
        let id = old.id;
        store.add(old).unwrap();
        store.prune(None).unwrap();
        assert!(!store.entries.iter().any(|e| e.id == id));
        assert_eq!(store.statistics.len(), 1);
        assert_eq!(store.statistics[0].words, 2);
    }
    #[test]
    fn legacy_history_is_migrated_before_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let mut entry = Entry::new("Microphone recording".into());
        entry.status = Status::Ready;
        entry.created = Utc::now() - chrono::Duration::days(2);
        entry.text = "legacy words".into();
        let mut raw = serde_json::to_value(&entry).unwrap();
        raw.as_object_mut().unwrap().remove("imported");
        raw.as_object_mut().unwrap().remove("app");
        let mut imported = raw.clone();
        imported["id"] = serde_json::to_value(Uuid::new_v4()).unwrap();
        imported["label"] = "old-import.wav".into();
        atomic_json(&dir.path().join("history.json"), &vec![raw, imported]).unwrap();
        let store = Store::open(dir.path().into()).unwrap();
        assert!(store.entries.is_empty());
        assert_eq!(store.statistics.len(), 1);
        assert_eq!(store.statistics[0].words, 2);
        drop(store);
        assert_eq!(Store::open(dir.path().into()).unwrap().statistics.len(), 1);
    }
    #[test]
    fn failed_statistics_write_preserves_recording() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().into()).unwrap();
        let mut entry = Entry::new("Microphone recording".into());
        entry.status = Status::Ready;
        entry.text = "keep this".into();
        let id = entry.id;
        store.entries.push(entry);
        fs::write(store.audio(id), b"audio").unwrap();
        fs::create_dir(dir.path().join("statistics.pending")).unwrap();
        assert!(store.delete(id).is_err());
        assert!(store.audio(id).exists());
        assert_eq!(store.entries.len(), 1);
    }
    #[test]
    fn failures_survive_restart_and_can_be_retried() {
        let dir = tempfile::tempdir().unwrap();
        let id;
        {
            let mut s = Store::open(dir.path().into()).unwrap();
            let e = Entry::new("Mic".into());
            id = e.id;
            s.add(e).unwrap();
            fs::write(s.audio(id), b"audio").unwrap();
        }
        let s = Store::open(dir.path().into()).unwrap();
        assert_eq!(s.entries[0].status, Status::Failed);
        assert!(s.audio(id).exists());
    }
    #[test]
    fn only_ten_audio_files_remain() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().into()).unwrap();
        for _ in 0..12 {
            let e = Entry::new("Mic".into());
            fs::write(s.audio(e.id), b"audio").unwrap();
            s.add(e).unwrap();
        }
        assert_eq!(s.entries.len(), 10);
        assert_eq!(fs::read_dir(s.root.join("recordings")).unwrap().count(), 10);
    }
    #[test]
    fn expired_audio_is_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().into()).unwrap();
        let mut e = Entry::new("Mic".into());
        e.created = Utc::now() - chrono::Duration::hours(25);
        let id = e.id;
        fs::write(s.audio(id), b"audio").unwrap();
        s.add(e).unwrap();
        s.prune(None).unwrap();
        assert!(!s.audio(id).exists());
        assert!(s.entries.is_empty());
    }
    #[test]
    fn corrupt_metadata_never_erases_audio() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("recordings")).unwrap();
        fs::write(dir.path().join("recordings/keep.wav"), b"audio").unwrap();
        fs::write(dir.path().join("history.json"), b"broken").unwrap();
        assert!(Store::open(dir.path().into()).is_err());
        assert!(dir.path().join("recordings/keep.wav").exists());
    }
    #[test]
    fn dictionary_is_literal_and_word_bounded() {
        let rules = vec![Rule {
            heard: "rust".into(),
            replacement: "Rust $1".into(),
        }];
        assert_eq!(
            apply_rules("rust, trusty RUST.", &rules),
            "Rust $1, trusty Rust $1."
        );
        assert_eq!(
            apply_rules("rust rust rust", &rules),
            "Rust $1 Rust $1 Rust $1"
        );
    }
}
