use std::{
    fs,
    process::{Command, Output},
};

const USAGE: &str = "usage: quiczk <quiz.toml>\n       quiczk fmt <quiz.toml>...\n       quiczk [ -v | --version | version ]\n       quiczk [ -h | --help | help ]\n";

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quiczk"))
        .args(arguments)
        .output()
        .expect("quiczk should start")
}

#[test]
fn no_arguments_print_usage_to_stderr_and_exit_two() {
    let output = run(&[]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, USAGE.as_bytes());
}

#[test]
fn help_aliases_print_usage_to_stdout_and_exit_zero() {
    for argument in ["help", "-h", "--help"] {
        let output = run(&[argument]);

        assert_eq!(output.status.code(), Some(0), "{argument}");
        assert_eq!(output.stdout, USAGE.as_bytes(), "{argument}");
        assert_eq!(output.stderr, b"", "{argument}");
    }
}

#[test]
fn version_aliases_print_version_to_stdout_and_exit_zero() {
    for argument in ["version", "-v", "--version"] {
        let output = run(&[argument]);

        assert_eq!(output.status.code(), Some(0), "{argument}");
        assert_eq!(
            output.stdout,
            format!("quiczk {}\n", env!("CARGO_PKG_VERSION")).as_bytes(),
            "{argument}"
        );
        assert_eq!(output.stderr, b"", "{argument}");
    }
}

#[test]
fn file_errors_do_not_depend_on_file_existence() {
    for argument in ["README.md", "missing.md"] {
        let output = run(&[argument]);
        let expected = format!("error: {argument}: not a .toml file\n");

        assert_eq!(output.status.code(), Some(1), "{argument}");
        assert_eq!(output.stdout, b"", "{argument}");
        assert_eq!(output.stderr, expected.as_bytes(), "{argument}");
    }
}

#[test]
fn argument_errors_identify_the_first_relevant_problem() {
    let output = run(&["example.toml", "example.toml"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        output.stderr,
        b"error: multiple quiz TOML paths provided\n\n"
            .iter()
            .chain(USAGE.as_bytes())
            .copied()
            .collect::<Vec<_>>()
    );

    let output = run(&["example.toml", "example.toml", "README.md"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stderr, b"error: README.md: not a .toml file\n");
}

#[test]
fn end_of_options_marker_is_supported() {
    let output = run(&["--", "README.md"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"error: README.md: not a .toml file\n");
}

#[test]
fn fmt_rewrites_the_quiz_in_place_without_stdout() {
    let path = std::env::temp_dir().join(format!("quiczk-fmt-{}.toml", std::process::id()));
    let path_string = path.to_str().expect("temporary path must be UTF-8");
    fs::write(
        &path,
        "[quiczk]\na1 = \"one\"\no1 = [\"one\", \"two\"]\nq1 = \"Question\"\n",
    )
    .expect("temporary quiz should be writable");

    let output = run(&["fmt", path_string]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"");
    assert_eq!(
        fs::read_to_string(&path).expect("formatted quiz should be readable"),
        "[quiczk]\nq1 = \"Question\"\no1 = [\n  \"one\",\n  \"two\",\n]\na1 = \"one\"\n"
    );

    fs::remove_file(path).expect("temporary quiz should be removable");
}

#[test]
fn fmt_rewrites_multiple_quizzes_in_place() {
    let temp_dir = std::env::temp_dir();
    let path1 = temp_dir.join(format!("quiczk-fmt-multi-1-{}.toml", std::process::id()));
    let path2 = temp_dir.join(format!("quiczk-fmt-multi-2-{}.toml", std::process::id()));

    fs::write(
        &path1,
        "[quiczk]\na1 = \"one\"\no1 = [\"one\", \"two\"]\nq1 = \"Question 1\"\n",
    )
    .expect("temporary quiz 1 should be writable");
    fs::write(
        &path2,
        "[quiczk]\na1 = \"two\"\no1 = [\"one\", \"two\"]\nq1 = \"Question 2\"\n",
    )
    .expect("temporary quiz 2 should be writable");

    let output = run(&[
        "fmt",
        path1.to_str().expect("valid path 1"),
        path2.to_str().expect("valid path 2"),
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"");
    assert_eq!(
        fs::read_to_string(&path1).expect("formatted quiz 1 should be readable"),
        "[quiczk]\nq1 = \"Question 1\"\no1 = [\n  \"one\",\n  \"two\",\n]\na1 = \"one\"\n"
    );
    assert_eq!(
        fs::read_to_string(&path2).expect("formatted quiz 2 should be readable"),
        "[quiczk]\nq1 = \"Question 2\"\no1 = [\n  \"one\",\n  \"two\",\n]\na1 = \"two\"\n"
    );

    fs::remove_file(path1).expect("temporary quiz 1 should be removable");
    fs::remove_file(path2).expect("temporary quiz 2 should be removable");
}
