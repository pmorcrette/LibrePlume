//! Persistent server state: settings, user dictionary and custom themes.
//!
//! Everything lives in small JSON files inside the data directory and is written
//! atomically (write to a temporary file, then rename).

use crate::themes::{builtin_themes, Theme};
use libreplume_core::{Category, CheckOptions, Language};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// Language used when a request does not specify one; `None` means auto-detect.
    pub default_language: Option<Language>,
    pub disabled_rules: BTreeSet<String>,
    pub enabled_rules: BTreeSet<String>,
    pub disabled_categories: BTreeSet<Category>,
    /// Sentences longer than this are reported (0 disables the rule).
    pub max_sentence_words: usize,
    /// Theme applied by the web interface for new visitors.
    pub default_theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            default_language: None,
            disabled_rules: BTreeSet::new(),
            enabled_rules: BTreeSet::new(),
            disabled_categories: BTreeSet::new(),
            max_sentence_words: libreplume_core::engine::DEFAULT_MAX_SENTENCE_WORDS,
            default_theme: "plume".into(),
        }
    }
}

/// Partial update of [`Settings`]; absent fields are left unchanged.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SettingsPatch {
    #[serde(default, deserialize_with = "double_option")]
    pub default_language: Option<Option<Language>>,
    pub disabled_rules: Option<BTreeSet<String>>,
    pub enabled_rules: Option<BTreeSet<String>>,
    pub disabled_categories: Option<BTreeSet<Category>>,
    pub max_sentence_words: Option<usize>,
    pub default_theme: Option<String>,
}

fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    settings: Settings,
    dictionary: BTreeSet<String>,
    themes: BTreeMap<String, Theme>,
}

#[derive(Debug)]
pub enum StoreError {
    NotFound(String),
    Invalid(String),
    Io(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::NotFound(s) => write!(f, "not found: {s}"),
            StoreError::Invalid(s) => write!(f, "invalid request: {s}"),
            StoreError::Io(s) => write!(f, "storage error: {s}"),
        }
    }
}

impl std::error::Error for StoreError {}

pub struct Store {
    dir: Option<PathBuf>,
    state: RwLock<State>,
}

const SETTINGS_FILE: &str = "settings.json";
const DICTIONARY_FILE: &str = "dictionary.json";
const THEMES_FILE: &str = "themes.json";
const MAX_WORD_LEN: usize = 80;
const MAX_DICTIONARY_WORDS: usize = 100_000;
const MAX_THEMES: usize = 200;

impl Store {
    /// Opens (or creates) the store in `dir`. `None` keeps everything in memory.
    pub fn open(dir: Option<&Path>) -> Result<Store, StoreError> {
        let mut state = State::default();
        if let Some(dir) = dir {
            std::fs::create_dir_all(dir).map_err(|e| StoreError::Io(format!("{}: {e}", dir.display())))?;
            if let Some(s) = read_json(&dir.join(SETTINGS_FILE))? {
                state.settings = s;
            }
            if let Some(d) = read_json(&dir.join(DICTIONARY_FILE))? {
                state.dictionary = d;
            }
            if let Some(t) = read_json(&dir.join(THEMES_FILE))? {
                state.themes = t;
            }
        }
        Ok(Store { dir: dir.map(Path::to_path_buf), state: RwLock::new(state) })
    }

    pub fn in_memory() -> Store {
        Store { dir: None, state: RwLock::new(State::default()) }
    }

    fn persist<T: Serialize>(&self, file: &str, value: &T) -> Result<(), StoreError> {
        let Some(dir) = &self.dir else { return Ok(()) };
        let path = dir.join(file);
        let tmp = dir.join(format!(".{file}.tmp"));
        let json = serde_json::to_vec_pretty(value).map_err(|e| StoreError::Io(e.to_string()))?;
        std::fs::write(&tmp, json).map_err(|e| StoreError::Io(format!("{}: {e}", tmp.display())))?;
        std::fs::rename(&tmp, &path).map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))
    }

    pub fn settings(&self) -> Settings {
        self.state.read().unwrap().settings.clone()
    }

    pub fn update_settings(&self, patch: SettingsPatch) -> Result<Settings, StoreError> {
        let mut state = self.state.write().unwrap();
        let mut s = state.settings.clone();
        if let Some(l) = patch.default_language {
            s.default_language = l;
        }
        if let Some(r) = patch.disabled_rules {
            s.disabled_rules = r;
        }
        if let Some(r) = patch.enabled_rules {
            s.enabled_rules = r;
        }
        if let Some(c) = patch.disabled_categories {
            s.disabled_categories = c;
        }
        if let Some(m) = patch.max_sentence_words {
            if m > 1000 {
                return Err(StoreError::Invalid("max_sentence_words must be at most 1000".into()));
            }
            s.max_sentence_words = m;
        }
        if let Some(t) = patch.default_theme {
            if !builtin_themes().iter().any(|b| b.id == t) && !state.themes.contains_key(&t) {
                return Err(StoreError::NotFound(format!("theme {t}")));
            }
            s.default_theme = t;
        }
        self.persist(SETTINGS_FILE, &s)?;
        state.settings = s.clone();
        Ok(s)
    }

    /// Enables or disables one rule, taking its default state into account.
    pub fn set_rule(&self, id: &str, enabled: bool) -> Result<Settings, StoreError> {
        let Some(rule) = libreplume_core::list_rules(None).into_iter().find(|r| r.id == id) else {
            return Err(StoreError::NotFound(format!("rule {id}")));
        };
        let mut s = self.settings();
        s.disabled_rules.remove(id);
        s.enabled_rules.remove(id);
        if enabled && !rule.enabled_by_default {
            s.enabled_rules.insert(id.to_string());
        } else if !enabled && rule.enabled_by_default {
            s.disabled_rules.insert(id.to_string());
        }
        self.update_settings(SettingsPatch {
            disabled_rules: Some(s.disabled_rules),
            enabled_rules: Some(s.enabled_rules),
            ..Default::default()
        })
    }

    pub fn rule_enabled(settings: &Settings, id: &str, default: bool) -> bool {
        if settings.disabled_rules.contains(id) {
            false
        } else {
            default || settings.enabled_rules.contains(id)
        }
    }

    pub fn dictionary(&self) -> Vec<String> {
        self.state.read().unwrap().dictionary.iter().cloned().collect()
    }

    pub fn add_words(&self, words: &[String]) -> Result<Vec<String>, StoreError> {
        let mut state = self.state.write().unwrap();
        let mut dict = state.dictionary.clone();
        for w in words {
            let w = w.trim();
            if w.is_empty() || w.chars().count() > MAX_WORD_LEN || w.chars().any(char::is_whitespace) {
                return Err(StoreError::Invalid(format!("« {w} » is not a single word")));
            }
            dict.insert(w.to_string());
        }
        if dict.len() > MAX_DICTIONARY_WORDS {
            return Err(StoreError::Invalid("dictionary is full".into()));
        }
        self.persist(DICTIONARY_FILE, &dict)?;
        state.dictionary = dict;
        Ok(state.dictionary.iter().cloned().collect())
    }

    pub fn remove_words(&self, words: &[String]) -> Result<Vec<String>, StoreError> {
        let mut state = self.state.write().unwrap();
        let mut dict = state.dictionary.clone();
        for w in words {
            dict.remove(w.trim());
        }
        self.persist(DICTIONARY_FILE, &dict)?;
        state.dictionary = dict;
        Ok(state.dictionary.iter().cloned().collect())
    }

    /// Built-in themes followed by custom ones.
    pub fn themes(&self) -> Vec<Theme> {
        let mut all = builtin_themes();
        all.extend(self.state.read().unwrap().themes.values().cloned());
        all
    }

    pub fn theme(&self, id: &str) -> Option<Theme> {
        self.themes().into_iter().find(|t| t.id == id)
    }

    pub fn save_theme(&self, mut theme: Theme) -> Result<Theme, StoreError> {
        theme.builtin = false;
        theme.validate().map_err(StoreError::Invalid)?;
        if builtin_themes().iter().any(|b| b.id == theme.id) {
            return Err(StoreError::Invalid(format!("« {} » is a built-in theme; choose another id", theme.id)));
        }
        let mut state = self.state.write().unwrap();
        let mut themes = state.themes.clone();
        themes.insert(theme.id.clone(), theme.clone());
        if themes.len() > MAX_THEMES {
            return Err(StoreError::Invalid("too many themes".into()));
        }
        self.persist(THEMES_FILE, &themes)?;
        state.themes = themes;
        Ok(theme)
    }

    pub fn delete_theme(&self, id: &str) -> Result<(), StoreError> {
        let mut state = self.state.write().unwrap();
        if !state.themes.contains_key(id) {
            return Err(StoreError::NotFound(format!("custom theme {id}")));
        }
        let mut themes = state.themes.clone();
        themes.remove(id);
        self.persist(THEMES_FILE, &themes)?;
        state.themes = themes;
        if state.settings.default_theme == id {
            let mut s = state.settings.clone();
            s.default_theme = "plume".into();
            self.persist(SETTINGS_FILE, &s)?;
            state.settings = s;
        }
        Ok(())
    }

    /// Check options from the stored settings, with per-request overrides.
    pub fn check_options(&self, overrides: &RequestOverrides) -> CheckOptions {
        let state = self.state.read().unwrap();
        let s = &state.settings;
        let mut o = CheckOptions {
            language: overrides.language.or(s.default_language),
            disabled_rules: s.disabled_rules.iter().cloned().collect(),
            enabled_rules: s.enabled_rules.iter().cloned().collect(),
            disabled_categories: s.disabled_categories.iter().copied().collect(),
            user_words: state.dictionary.iter().cloned().collect(),
            max_sentence_words: Some(overrides.max_sentence_words.unwrap_or(s.max_sentence_words)),
        };
        for r in &overrides.disabled_rules {
            o.enabled_rules.remove(r);
            o.disabled_rules.insert(r.clone());
        }
        for r in &overrides.enabled_rules {
            o.disabled_rules.remove(r);
            o.enabled_rules.insert(r.clone());
        }
        if let Some(cats) = &overrides.categories {
            o.disabled_categories = Category::ALL.into_iter().filter(|c| !cats.contains(c)).collect();
        }
        o.disabled_categories.extend(overrides.disabled_categories.iter().copied());
        o.user_words.extend(overrides.extra_words.iter().cloned());
        o
    }
}

/// Options a single request may add on top of the stored settings.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RequestOverrides {
    pub language: Option<Language>,
    pub disabled_rules: Vec<String>,
    pub enabled_rules: Vec<String>,
    /// Only check these categories.
    pub categories: Option<Vec<Category>>,
    pub disabled_categories: Vec<Category>,
    pub max_sentence_words: Option<usize>,
    /// Extra accepted words for this request only.
    pub extra_words: Vec<String>,
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>, StoreError> {
    match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map(Some).map_err(|e| StoreError::Io(format!("{}: {e}", path.display())))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(StoreError::Io(format!("{}: {e}", path.display()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_to_disk() {
        let dir = std::env::temp_dir().join(format!("libreplume-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        {
            let store = Store::open(Some(&dir)).unwrap();
            store.add_words(&["zorglub".into()]).unwrap();
            store.set_rule("LONG_SENTENCE", false).unwrap();
            store.set_rule("ELLIPSIS", true).unwrap();
        }
        let store = Store::open(Some(&dir)).unwrap();
        assert_eq!(store.dictionary(), vec!["zorglub"]);
        let s = store.settings();
        assert!(s.disabled_rules.contains("LONG_SENTENCE"));
        assert!(s.enabled_rules.contains("ELLIPSIS"));
        assert!(store.set_rule("NOPE", true).is_err());
        assert!(store.add_words(&["two words".into()]).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
