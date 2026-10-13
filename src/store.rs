use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const KEEP: usize = 5;
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
            serde_json::from_slice(&fs::read(root.join("history.json"))?)
                .context("History is damaged; your recordings have not been removed.")?
        } else {
            vec![]
        };
        let mut store = Self {
            root,
            settings,
            entries,
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
    pub fn add(&mut self, entry: Entry) -> Result<()> {
        let id = entry.id;
        self.entries.insert(0, entry);
        if let Err(error) = self.persist() {
            self.entries.remove(0);
            return Err(error);
        }
        self.prune(Some(id))
    }
    pub fn prune(&mut self, active: Option<Uuid>) -> Result<()> {
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
    fn only_five_audio_files_remain() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(dir.path().into()).unwrap();
        for _ in 0..7 {
            let e = Entry::new("Mic".into());
            fs::write(s.audio(e.id), b"audio").unwrap();
            s.add(e).unwrap();
        }
        assert_eq!(s.entries.len(), 5);
        assert_eq!(fs::read_dir(s.root.join("recordings")).unwrap().count(), 5);
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
