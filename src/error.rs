use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum AppError {
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

    pub fn terminal(source: io::Error) -> Self {
        Self::Terminal(source)
    }
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
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

#[cfg(test)]
mod tests {
    use super::AppError;
    use std::{error::Error as _, io, path::Path};

    #[test]
    fn source_exposes_each_wrapped_error() {
        let path = Path::new("quiz.toml");

        let io_error = AppError::io(path, io::Error::other("io boom"));
        assert_eq!(
            io_error.source().expect("io has source").to_string(),
            "io boom"
        );

        let toml_error = toml::from_str::<toml::Table>("= invalid").expect_err("must not parse");
        let toml_error = AppError::toml(path, toml_error);
        assert!(toml_error.source().is_some());

        let terminal = AppError::terminal(io::Error::other("term boom"));
        assert_eq!(
            terminal.source().expect("terminal has source").to_string(),
            "term boom"
        );

        let validation = AppError::validation(path, "bad");
        assert!(validation.source().is_none());
    }
}
