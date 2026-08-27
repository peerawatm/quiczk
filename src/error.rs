use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum AppError {
    InvalidConfigPath,
    Terminal(io::Error),
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },
    Validation {
        path: PathBuf,
        message: String,
    },
}

impl AppError {
    pub fn io(path: &Path, source: io::Error) -> Self {
        Self::Io {
            path: path.to_owned(),
            source,
        }
    }

    pub fn toml(path: &Path, source: toml::de::Error) -> Self {
        Self::Toml {
            path: path.to_owned(),
            source,
        }
    }

    pub fn validation(path: &Path, message: impl Into<String>) -> Self {
        Self::Validation {
            path: path.to_owned(),
            message: message.into(),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfigPath => write!(formatter, "cannot determine the config directory"),
            Self::Terminal(source) => write!(formatter, "terminal: {source}"),
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Toml { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Validation { path, message } => {
                write!(formatter, "{}: {message}", path.display())
            }
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Toml { source, .. } => Some(source),
            Self::Terminal(source) => Some(source),
            _ => None,
        }
    }
}

impl AppError {
    pub fn terminal(source: io::Error) -> Self {
        Self::Terminal(source)
    }
}
