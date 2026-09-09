mod app;
mod config;

use app::App;
use config::Config;
use quiczk::{error::AppError, quiz::Quiz};
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process,
};

const USAGE: &str = "usage: quiczk <quiz.toml>\n       quiczk fmt <quiz.toml>...\n       quiczk [ -v | --version | version ]\n       quiczk [ -h | --help | help ]";

fn main() {
    match parse_arguments(&env::args_os().skip(1).collect::<Vec<_>>()) {
        Ok(Action::Help) => print_help(),
        Ok(Action::Version) => print_version(),
        Ok(Action::Quiz(quiz_path)) => {
            if let Err(error) = run(quiz_path) {
                eprintln!("error: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Format(quiz_paths)) => {
            for quiz_path in &quiz_paths {
                if let Err(error) = Quiz::format_in_place(quiz_path) {
                    eprintln!("error: {error}");
                    process::exit(1);
                }
            }
        }
        Err(error) => error.exit(),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Help,
    Version,
    Quiz(PathBuf),
    Format(Vec<PathBuf>),
}

#[derive(Debug, PartialEq, Eq)]
enum CliError {
    Usage,
    UnknownCommand(PathBuf),
    NotToml(PathBuf),
    MultipleQuizPaths,
    UnexpectedArgument(PathBuf),
}

impl CliError {
    fn exit(self) -> ! {
        let exit_code = match &self {
            Self::NotToml(_) => 1,
            _ => 2,
        };

        match self {
            Self::Usage => eprintln!("{USAGE}"),
            Self::UnknownCommand(command) => {
                eprintln!("error: unknown command `{}`\n\n{USAGE}", command.display())
            }
            Self::NotToml(path) => eprintln!("error: {}: not a .toml file", path.display()),
            Self::MultipleQuizPaths => {
                eprintln!("error: multiple quiz TOML paths provided\n\n{USAGE}")
            }
            Self::UnexpectedArgument(argument) => eprintln!(
                "error: unexpected argument `{}`\n\n{USAGE}",
                argument.display()
            ),
        }

        process::exit(exit_code)
    }
}

fn parse_arguments(arguments: &[OsString]) -> Result<Action, CliError> {
    let (args, options_ended) = if arguments.first().is_some_and(|arg| arg.as_os_str() == "--") {
        (&arguments[1..], true)
    } else {
        (arguments, false)
    };

    let Some(first) = args.first() else {
        return Err(CliError::Usage);
    };

    if !options_ended {
        match first.to_str() {
            Some("help" | "-h" | "--help") => {
                return if args.len() == 1 {
                    Ok(Action::Help)
                } else {
                    Err(CliError::UnexpectedArgument(PathBuf::from(&args[1])))
                };
            }
            Some("version" | "-v" | "--version") => {
                return if args.len() == 1 {
                    Ok(Action::Version)
                } else {
                    Err(CliError::UnexpectedArgument(PathBuf::from(&args[1])))
                };
            }
            _ => {}
        }
        if first == "fmt" {
            if args.len() == 1 {
                return Err(CliError::Usage);
            }
            let mut quiz_paths = Vec::with_capacity(args.len() - 1);
            for arg in &args[1..] {
                let path = PathBuf::from(arg);
                if !is_toml_path(&path) {
                    return Err(if is_file_like(&path) {
                        CliError::NotToml(path)
                    } else {
                        CliError::UnexpectedArgument(path)
                    });
                }
                quiz_paths.push(path);
            }
            return Ok(Action::Format(quiz_paths));
        }
    }

    let quiz_path = PathBuf::from(first);
    if !is_toml_path(&quiz_path) {
        return Err(if is_file_like(&quiz_path) {
            CliError::NotToml(quiz_path)
        } else {
            CliError::UnknownCommand(quiz_path)
        });
    }

    for arg in &args[1..] {
        let path = PathBuf::from(arg);
        if !is_toml_path(&path) {
            return Err(if is_file_like(&path) {
                CliError::NotToml(path)
            } else {
                CliError::UnexpectedArgument(path)
            });
        }
    }
    if args.len() > 1 {
        return Err(CliError::MultipleQuizPaths);
    }

    Ok(Action::Quiz(quiz_path))
}

fn is_toml_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
}

fn is_file_like(path: &Path) -> bool {
    path.extension().is_some() || path.components().count() > 1
}

fn run(quiz_path: PathBuf) -> Result<(), AppError> {
    let quiz = Quiz::load(&quiz_path)?;
    let config = Config::resolve(&quiz_path, quiz.config.as_ref())?;
    let title = quiz_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("quiz")
        .to_owned();
    let app = App::new(quiz, config, title);

    app::run(app).map_err(AppError::terminal)
}

fn print_help() {
    println!("{USAGE}");
}

fn print_version() {
    println!("quiczk {}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn parser_reports_missing_arguments() {
        assert_eq!(parse_arguments(&[]), Err(CliError::Usage));
    }

    #[test]
    fn parser_accepts_help_and_version_aliases() {
        assert_eq!(parse_arguments(&arguments(&["help"])), Ok(Action::Help));
        assert_eq!(
            parse_arguments(&arguments(&["--version"])),
            Ok(Action::Version)
        );
    }

    #[test]
    fn parser_accepts_the_fmt_subcommand() {
        assert_eq!(
            parse_arguments(&arguments(&["fmt", "quiz.toml"])),
            Ok(Action::Format(vec![PathBuf::from("quiz.toml")]))
        );
        assert_eq!(
            parse_arguments(&arguments(&["fmt", "quiz1.toml", "quiz2.toml"])),
            Ok(Action::Format(vec![
                PathBuf::from("quiz1.toml"),
                PathBuf::from("quiz2.toml"),
            ]))
        );
        assert_eq!(parse_arguments(&arguments(&["fmt"])), Err(CliError::Usage));
    }

    #[test]
    fn parser_rejects_extra_arguments_to_control_commands() {
        assert_eq!(
            parse_arguments(&arguments(&["help", "example.toml"])),
            Err(CliError::UnexpectedArgument(PathBuf::from("example.toml")))
        );
    }

    #[test]
    fn parser_does_not_use_filesystem_existence_to_classify_paths() {
        assert_eq!(
            parse_arguments(&arguments(&["missing.md"])),
            Err(CliError::NotToml(PathBuf::from("missing.md")))
        );
        assert_eq!(
            parse_arguments(&arguments(&["gay"])),
            Err(CliError::UnknownCommand(PathBuf::from("gay")))
        );
    }

    #[test]
    fn parser_checks_all_extra_arguments_before_reporting_multiple_paths() {
        assert_eq!(
            parse_arguments(&arguments(&["example.toml", "example.toml"])),
            Err(CliError::MultipleQuizPaths)
        );
        assert_eq!(
            parse_arguments(&arguments(&["example.toml", "example.toml", "README.md"])),
            Err(CliError::NotToml(PathBuf::from("README.md")))
        );
    }

    #[test]
    fn parser_supports_end_of_options_marker() {
        assert_eq!(
            parse_arguments(&arguments(&["--", "example.toml"])),
            Ok(Action::Quiz(PathBuf::from("example.toml")))
        );
        assert_eq!(
            parse_arguments(&arguments(&["--", "--help"])),
            Err(CliError::UnknownCommand(PathBuf::from("--help")))
        );
    }
}
