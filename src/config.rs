use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use quiczk::error::AppError;

#[derive(Debug, Deserialize)]
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
        let path = config_path()?;

        match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str::<ConfigFile>(&contents)
                .map(|file| file.config)
                .map_err(|source| AppError::toml(&path, source)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(AppError::io(&path, source)),
        }
    }
}

fn config_path() -> Result<PathBuf, AppError> {
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".config")))
        .ok_or(AppError::InvalidConfigPath)?;

    Ok(config_home.join("quiczk").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::{Config, ConfigFile};

    #[test]
    fn defaults_random_start_cursor_to_true() {
        assert!(Config::default().random_start_cursor);
        assert_eq!(Config::default().question_timer_seconds, None);
        assert!(Config::default().hide_options_until_interact);
    }

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
        let file =
            toml::from_str::<ConfigFile>("[config]\nhide_options_until_interact = false\n")
                .expect("config should parse");

        assert!(!file.config.hide_options_until_interact);
    }
}
