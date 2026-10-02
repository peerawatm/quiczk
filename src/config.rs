use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use quiczk::error::AppError;

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub shuffle_questions: bool,
    pub shuffle_options: bool,
    pub random_start_cursor: bool,
    pub question_timer_seconds: Option<u64>,
    pub hide_options_until_interact: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ConfigFile {
    config: Config,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shuffle_questions: false,
            shuffle_options: true,
            random_start_cursor: true,
            question_timer_seconds: None,
            hide_options_until_interact: true,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, AppError> {
        let Some(path) = config_path() else {
            return Ok(Self::default());
        };

        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str::<ConfigFile>(&contents)
                .map(|file| file.config)
                .map_err(|source| AppError::toml(&path, source)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(AppError::io(&path, source)),
        }
    }

    /// Resolve the effective config with precedence: embedded quiz table,
    /// then XDG config file, then compiled defaults.
    ///
    /// The overlay is a generic key-wise table merge, so future keys need
    /// no parser changes here. Unknown keys fail strictly, attributed to
    /// the quiz file whenever an embedded table is present.
    pub fn resolve(quiz_path: &Path, embedded: Option<&Table>) -> Result<Self, AppError> {
        let base = Self::load()?;
        let Some(embedded) = embedded else {
            return Ok(base);
        };
        // Strict validation first so typos blame the quiz file.
        Self::from_table(embedded, quiz_path)?;
        let mut merged = match Value::try_from(&base).expect("Config must serialize") {
            Value::Table(table) => table,
            _ => unreachable!("Config must serialize as a table"),
        };
        overlay(&mut merged, embedded);
        Self::from_table(&merged, quiz_path)
    }

    fn from_table(table: &Table, path: &Path) -> Result<Self, AppError> {
        Self::deserialize(Value::Table(table.clone()))
            .map_err(|source| AppError::toml(path, source))
    }
}

/// Key-wise overlay. Later layers win per key, so partial tables fall
/// through to lower-precedence values without per-key code.
fn overlay(base: &mut Table, over: &Table) {
    for (key, value) in over {
        base.insert(key.clone(), value.clone());
    }
}

fn config_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        return Some(path.join("quiczk").join("config.toml"));
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        return Some(home.join(".config").join("quiczk").join("config.toml"));
    }
    if let Some(profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
        return Some(profile.join(".config").join("quiczk").join("config.toml"));
    }
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|base| base.join("quiczk").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::{Config, ConfigFile, overlay};
    use toml::{Table, Value};

    #[test]
    fn random_start_cursor_can_be_disabled() {
        let file = toml::from_str::<ConfigFile>("[config]\nrandom_start_cursor = false\n")
            .expect("config should parse");

        assert!(!file.config.random_start_cursor);
    }

    #[test]
    fn question_timer_seconds_can_be_configured() {
        let file = toml::from_str::<ConfigFile>("[config]\nquestion_timer_seconds = 15\n")
            .expect("config should parse");

        assert_eq!(file.config.question_timer_seconds, Some(15));
    }

    #[test]
    fn hide_options_until_interact_can_be_disabled() {
        let file = toml::from_str::<ConfigFile>("[config]\nhide_options_until_interact = false\n")
            .expect("config should parse");

        assert!(!file.config.hide_options_until_interact);
    }

    #[test]
    fn embedded_timer_overrides_base_without_clobbering() {
        let base = Config {
            shuffle_options: false,
            ..Config::default()
        };
        let mut merged = match Value::try_from(&base).expect("base must serialize") {
            Value::Table(table) => table,
            _ => panic!("base must serialize as a table"),
        };
        let embedded: Table =
            toml::from_str("question_timer_seconds = 30").expect("embedded must parse");
        overlay(&mut merged, &embedded);
        let resolved = Config::from_table(&merged, std::path::Path::new("quiz.toml"))
            .expect("merged config must validate");

        assert_eq!(resolved.question_timer_seconds, Some(30));
        assert!(!resolved.shuffle_options);
        assert!(!resolved.shuffle_questions);
    }

    #[test]
    fn timer_zero_survives_merge_for_app_normalization() {
        let embedded: Table =
            toml::from_str("question_timer_seconds = 0").expect("embedded must parse");
        let resolved = Config::from_table(&embedded, std::path::Path::new("quiz.toml"))
            .expect("zero timer must parse");

        assert_eq!(resolved.question_timer_seconds, Some(0));
    }

    #[test]
    fn unknown_embedded_key_fails_strictly() {
        let embedded: Table = toml::from_str("bogus_key = true").expect("embedded must parse");
        let error = Config::from_table(&embedded, std::path::Path::new("quiz.toml"))
            .expect_err("unknown keys must fail");

        assert!(error.to_string().contains("quiz.toml"));
    }
}
