use std::{collections::BTreeSet, fmt::Write, fs, path::Path};

use toml::{Table, Value};

use crate::error::AppError;

#[derive(Debug)]
pub struct Quiz {
    pub questions: Vec<Question>,
    pub answers: Vec<Answer>,
}

#[derive(Debug)]
pub struct Question {
    pub question: String,
    pub options: Vec<String>,
    pub explanation: Option<String>,
}

#[derive(Debug)]
pub struct Answer {
    pub answer: String,
}

impl Quiz {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let is_toml = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"));
        if !is_toml {
            return Err(AppError::validation(path, "not a .toml file"));
        }

        let contents = fs::read_to_string(path).map_err(|source| AppError::io(path, source))?;
        Self::from_source(path, &contents)
    }

    pub fn from_source(path: &Path, source: &str) -> Result<Self, AppError> {
        let document: Table =
            toml::from_str(source).map_err(|source| AppError::toml(path, source))?;

        Self::from_document(path, source, document)
    }

    pub fn format_in_place(path: &Path) -> Result<(), AppError> {
        let quiz = Self::load(path)?;
        fs::write(path, quiz.format_source()).map_err(|source| AppError::io(path, source))
    }

    pub fn format_source(&self) -> String {
        let mut formatted = String::from("[quiczk]\n");
        for (index, (question, answer)) in self.questions.iter().zip(&self.answers).enumerate() {
            let number = index + 1;
            let _ = writeln!(formatted, "q{number} = {}", quoted(&question.question));
            let _ = writeln!(formatted, "o{number} = [");
            for option in &question.options {
                let _ = writeln!(formatted, "  {},", quoted(option));
            }
            formatted.push_str("]\n");
            let _ = writeln!(formatted, "a{number} = {}", quoted(&answer.answer));
            if let Some(explanation) = &question.explanation {
                let _ = writeln!(formatted, "e{number} = {}", quoted(explanation));
            }
            if number != self.questions.len() {
                formatted.push('\n');
            }
        }
        formatted
    }

    fn from_document(path: &Path, source: &str, document: Table) -> Result<Self, AppError> {
        for key in document.keys() {
            if key != "quiczk" {
                return Err(AppError::validation(
                    path,
                    format!("unexpected top-level table `{key}`"),
                ));
            }
        }

        let quiz = section(&document, "quiczk", path)?;
        validate_key_prefixes(quiz, &["q", "o", "a", "e"], "quiczk", path)?;
        let question_numbers = numbered_keys(quiz, "q", "quiczk", path)?;
        let option_numbers = numbered_keys(quiz, "o", "quiczk", path)?;
        let answer_numbers = numbered_keys(quiz, "a", "quiczk", path)?;
        let explanation_numbers = numbered_keys(quiz, "e", "quiczk", path)?;

        if question_numbers.is_empty() && option_numbers.is_empty() && answer_numbers.is_empty() {
            return Err(AppError::validation(path, "[quiczk] must not be empty"));
        }
        validate_matching_numbers(&question_numbers, &option_numbers, &answer_numbers, path)?;
        validate_contiguous(&question_numbers, "q", path)?;

        for number in &explanation_numbers {
            if !question_numbers.contains(number) {
                return Err(AppError::validation(
                    path,
                    format!("[quiczk] e{number} is provided without matching q{number}"),
                ));
            }
        }

        let mut parsed_questions = Vec::with_capacity(question_numbers.len());
        let mut parsed_answers = Vec::with_capacity(answer_numbers.len());
        let mut answer_errors = Vec::new();

        for number in question_numbers {
            let question_key = format!("q{number}");
            let option_key = format!("o{number}");
            let answer_key = format!("a{number}");
            let explanation_key = format!("e{number}");
            let question = string_value(quiz, &question_key, path)?;
            let options = options_value(quiz, &option_key, path)?;
            let answer = string_value(quiz, &answer_key, path)?;
            let explanation = if quiz.contains_key(&explanation_key) {
                Some(string_value(quiz, &explanation_key, path)?)
            } else {
                None
            };

            if !options.iter().any(|option| option == &answer) {
                let option_location = locate_key(source, "quiczk", &option_key);
                let answer_location = locate_key(source, "quiczk", &answer_key);
                answer_errors.push((option_location, answer_location));
            }

            parsed_questions.push(Question {
                question,
                options,
                explanation,
            });
            parsed_answers.push(Answer { answer });
        }

        if !answer_errors.is_empty() {
            let count = answer_errors.len();
            let mut report = format!(
                "{count} invalid answer{}",
                if count == 1 { "" } else { "s" }
            );
            let max_line = answer_errors
                .iter()
                .map(|(option_location, answer_location)| {
                    option_location
                        .as_ref()
                        .map(|location| location.0)
                        .into_iter()
                        .chain(answer_location.as_ref().map(|location| location.0))
                        .max()
                        .unwrap_or(1)
                })
                .max()
                .unwrap_or(1);
            let width = max_line.to_string().len();
            for (option_location, answer_location) in answer_errors {
                report.push_str(&source_excerpt(option_location, answer_location, width));
            }
            return Err(AppError::validation(path, report));
        }

        Ok(Self {
            questions: parsed_questions,
            answers: parsed_answers,
        })
    }
}

fn quoted(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');

    for character in value.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\u{8}' => quoted.push_str("\\b"),
            '\t' => quoted.push_str("\\t"),
            '\n' => quoted.push_str("\\n"),
            '\u{c}' => quoted.push_str("\\f"),
            '\r' => quoted.push_str("\\r"),
            '\u{1b}' => quoted.push_str("\\e"),
            character if character <= '\u{1f}' || character == '\u{7f}' => {
                let _ = write!(quoted, "\\u{:04X}", character as u32);
            }
            character => quoted.push(character),
        }
    }

    quoted.push('"');
    quoted
}

fn section<'a>(document: &'a Table, name: &str, path: &Path) -> Result<&'a Table, AppError> {
    document
        .get(name)
        .and_then(Value::as_table)
        .ok_or_else(|| AppError::validation(path, format!("[{}] table is required", name)))
}

fn numbered_keys(
    table: &Table,
    prefix: &str,
    section: &str,
    path: &Path,
) -> Result<BTreeSet<usize>, AppError> {
    let mut numbers = BTreeSet::new();

    for key in table.keys() {
        let Some(number) = key.strip_prefix(prefix) else {
            continue;
        };
        let number = number.parse::<usize>().map_err(|_| {
            AppError::validation(path, format!("{section}.{key} must end with a number"))
        })?;
        if number == 0 {
            return Err(AppError::validation(
                path,
                format!("{section}.{key} must start at 1"),
            ));
        }
        if !numbers.insert(number) {
            return Err(AppError::validation(
                path,
                format!("{section} contains duplicate numeric key {prefix}{number}"),
            ));
        }
    }

    Ok(numbers)
}

fn validate_matching_numbers(
    question_numbers: &BTreeSet<usize>,
    option_numbers: &BTreeSet<usize>,
    answer_numbers: &BTreeSet<usize>,
    path: &Path,
) -> Result<(), AppError> {
    let numbers = question_numbers
        .iter()
        .chain(option_numbers)
        .chain(answer_numbers)
        .copied()
        .collect::<BTreeSet<_>>();

    for number in numbers {
        let keys = [
            (format!("q{number}"), question_numbers.contains(&number)),
            (format!("o{number}"), option_numbers.contains(&number)),
            (format!("a{number}"), answer_numbers.contains(&number)),
        ];
        let missing = keys
            .iter()
            .filter(|(_, present)| !present)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        if missing.is_empty() {
            continue;
        }

        let present = keys
            .iter()
            .filter(|(_, present)| *present)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        let verb = if missing.len() == 1 { "is" } else { "are" };
        return Err(AppError::validation(
            path,
            format!(
                "[quiczk] {} {verb} required for {}",
                missing.join(" and "),
                present.join(" and ")
            ),
        ));
    }

    Ok(())
}

fn validate_contiguous(
    numbers: &BTreeSet<usize>,
    prefix: &str,
    path: &Path,
) -> Result<(), AppError> {
    for (index, number) in numbers.iter().enumerate() {
        let expected = index + 1;
        if *number != expected {
            let missing = if expected + 1 == *number {
                format!("{prefix}{expected}")
            } else {
                format!("{prefix}{expected} through {prefix}{}", number - 1)
            };
            return Err(AppError::validation(
                path,
                format!("[quiczk] missing {missing} before {prefix}{number}"),
            ));
        }
    }

    Ok(())
}

fn validate_key_prefixes(
    table: &Table,
    prefixes: &[&str],
    section: &str,
    path: &Path,
) -> Result<(), AppError> {
    for key in table.keys() {
        if !prefixes.iter().any(|prefix| key.starts_with(prefix)) {
            return Err(AppError::validation(
                path,
                format!("{section}.{key} uses an unsupported key name"),
            ));
        }
    }
    Ok(())
}

fn string_value(table: &Table, key: &str, path: &Path) -> Result<String, AppError> {
    let value = table
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::validation(path, format!("{key} must be a string")))?;
    if value.trim().is_empty() {
        return Err(AppError::validation(
            path,
            format!("{key} must not be empty"),
        ));
    }
    Ok(value.to_owned())
}

fn options_value(table: &Table, key: &str, path: &Path) -> Result<Vec<String>, AppError> {
    let options = table
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::validation(path, format!("{key} must be an array")))?;
    if options.len() < 2 {
        return Err(AppError::validation(
            path,
            format!("{key} must contain at least two options"),
        ));
    }

    let mut seen = BTreeSet::new();
    let mut parsed = Vec::with_capacity(options.len());
    for (index, option) in options.iter().enumerate() {
        let value = option
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .ok_or_else(|| {
                AppError::validation(path, format!("{key}[{index}] must be a non-empty string"))
            })?;
        if !seen.insert(value.clone()) {
            return Err(AppError::validation(
                path,
                format!("{key} contains duplicate option `{value}`"),
            ));
        }
        parsed.push(value);
    }

    Ok(parsed)
}

fn locate_key(source: &str, section: &str, key: &str) -> Option<(usize, usize, String)> {
    let mut current_section = None;

    for (line_index, source_line) in source.lines().enumerate() {
        let trimmed = source_line.trim();
        if let Some(name) = trimmed
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            current_section = Some(name);
            continue;
        }

        if current_section != Some(section) {
            continue;
        }

        let Some((left, _)) = source_line.split_once('=') else {
            continue;
        };
        if left.trim() != key {
            continue;
        }

        let column = source_line.find(key).map(|index| index + 1).unwrap_or(1);
        return Some((line_index + 1, column, source_line.trim().to_owned()));
    }

    None
}

fn source_excerpt(
    option_location: Option<(usize, usize, String)>,
    answer_location: Option<(usize, usize, String)>,
    width: usize,
) -> String {
    match (option_location, answer_location) {
        (Some((option_line, _, option_source)), Some((answer_line, _, answer_source))) => {
            format!(
                "\n{option_line:>width$} │ {option_source}\n{answer_line:>width$} │ {answer_source}",
                width = width
            )
        }
        (Some((option_line, _, option_source)), None) => {
            format!("\n{option_line:>width$} │ {option_source}", width = width)
        }
        (None, Some((answer_line, _, answer_source))) => {
            format!("\n{answer_line:>width$} │ {answer_source}", width = width)
        }
        (None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Result<Quiz, AppError> {
        let document: Table = toml::from_str(source).expect("test TOML must parse");
        Quiz::from_document(Path::new("quiz.toml"), source, document)
    }

    #[test]
    fn parses_compact_quiczk_table() {
        let source = r#"
            [quiczk]
            q1 = "What is the default shell on macOS?"
            o1 = ["bash", "zsh", "fish"]
            a1 = "zsh"
            q2 = "What command lists files?"
            o2 = ["ls", "cd", "pwd"]
            a2 = "ls"
            "#;
        let quiz = parse(source).expect("compact quiz must validate");

        assert_eq!(quiz.questions.len(), 2);
        assert_eq!(quiz.answers[1].answer, "ls");
    }

    #[test]
    fn loads_the_valid_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/valid.toml");
        let quiz = Quiz::load(&path).expect("valid quiz fixture must load");

        assert_eq!(quiz.questions.len(), 2);
        assert_eq!(quiz.answers[0].answer, "zsh");
    }

    #[test]
    fn rejects_non_toml_quiz_paths_before_reading() {
        let error = Quiz::load(Path::new("README.md")).expect_err("non-TOML paths must fail");

        assert_eq!(error.to_string(), "README.md: not a .toml file");
    }

    #[test]
    fn rejects_the_legacy_two_table_format() {
        let source = r#"
            [questions]
            q1 = "Question"
            o1 = ["one", "two"]

            [answers]
            a1 = "one"
            "#;

        let error = parse(source).expect_err("legacy schema must fail");

        assert!(error.to_string().contains("unexpected top-level table"));
    }

    #[test]
    fn rejects_mismatched_question_and_answer_numbers() {
        let source = r#"
            [quiczk]
            q1 = "Question"
            o1 = ["one", "two"]
            a2 = "one"
            "#;

        let error = parse(source).expect_err("mismatched numbers must fail");

        assert!(error.to_string().contains("a1 is required for q1 and o1"));
    }

    #[test]
    fn reports_missing_question_for_options_and_answer() {
        let source = r#"
            [quiczk]
            o1 = ["one", "two"]
            a1 = "one"
            "#;

        let error = parse(source).expect_err("missing question must fail");

        assert!(error.to_string().contains("q1 is required for o1 and a1"));
    }

    #[test]
    fn reports_missing_options_for_question_and_answer() {
        let source = r#"
            [quiczk]
            q1 = "Question"
            a1 = "one"
            "#;

        let error = parse(source).expect_err("missing options must fail");

        assert!(error.to_string().contains("o1 is required for q1 and a1"));
    }

    #[test]
    fn rejects_gapped_question_numbers() {
        let source = r#"
            [quiczk]
            q1 = "Question 1"
            o1 = ["one", "two"]
            a1 = "one"
            q3 = "Question 3"
            o3 = ["three", "four"]
            a3 = "three"
            "#;

        let error = parse(source).expect_err("gapped numbers must fail");

        assert!(error.to_string().contains("missing q2 before q3"));
    }

    #[test]
    fn reports_a_range_of_missing_question_numbers() {
        let source = r#"
            [quiczk]
            q1 = "Question 1"
            o1 = ["one", "two"]
            a1 = "one"
            q11 = "Question 11"
            o11 = ["eleven", "twelve"]
            a11 = "eleven"
            "#;

        let error = parse(source).expect_err("gapped numbers must fail");

        assert!(
            error
                .to_string()
                .contains("missing q2 through q10 before q11")
        );
    }

    #[test]
    fn rejects_duplicate_options() {
        let source = r#"
            [quiczk]
            q1 = "Question"
            o1 = ["one", "one"]
            a1 = "one"
            "#;

        let error = parse(source).expect_err("duplicate options must fail");

        assert!(
            error
                .to_string()
                .contains("o1 contains duplicate option `one`")
        );
    }

    #[test]
    fn rejects_unknown_quiz_keys() {
        let source = r#"
            [quiczk]
            title = "Unsupported metadata"
            q1 = "Question"
            o1 = ["one", "two"]
            a1 = "one"
            "#;

        let error = parse(source).expect_err("unknown keys must fail");

        assert!(
            error
                .to_string()
                .contains("title uses an unsupported key name")
        );
    }

    #[test]
    fn rejects_single_option_questions() {
        let source = r#"
            [quiczk]
            q1 = "Question"
            o1 = ["only option"]
            a1 = "only option"
            "#;

        let error = parse(source).expect_err("single-option questions must fail");

        assert!(
            error
                .to_string()
                .contains("o1 must contain at least two options")
        );
    }

    #[test]
    fn reports_all_answers_that_do_not_match_their_options() {
        let source = r#"
            [quiczk]
            q1 = "Question 1"
            o1 = ["one", "one alternative"]
            a1 = "wrong one"
            q2 = "Question 2"
            o2 = ["two", "two alternative"]
            a2 = "wrong two"
            "#;
        let error = parse(source).expect_err("invalid answers must fail");
        let message = error.to_string();

        assert!(message.contains("a1 = \"wrong one\""));
        assert!(message.contains("a2 = \"wrong two\""));
    }

    #[test]
    fn formats_questions_as_grouped_multiline_blocks() {
        let source = r#"
            [quiczk]
            a2 = "four"
            o2 = ["three", "four"]
            q2 = "Second question"
            q1 = "First question"
            a1 = "one"
            o1 = ["one", "two"]
            "#;
        let quiz = parse(source).expect("quiz must validate");

        let expected = concat!(
            "[quiczk]\n",
            "q1 = \"First question\"\n",
            "o1 = [\n",
            "  \"one\",\n",
            "  \"two\",\n",
            "]\n",
            "a1 = \"one\"\n\n",
            "q2 = \"Second question\"\n",
            "o2 = [\n",
            "  \"three\",\n",
            "  \"four\",\n",
            "]\n",
            "a2 = \"four\"\n",
        );

        assert_eq!(quiz.format_source(), expected);
        let reformatted = parse(&quiz.format_source())
            .expect("formatted quiz must validate")
            .format_source();
        assert_eq!(reformatted, expected);
    }

    #[test]
    fn formats_strings_with_toml_escapes() {
        let source = r#"
            [quiczk]
            q1 = "A \"quoted\" line\\path"
            o1 = ["first\nline", "second"]
            a1 = "first\nline"
            "#;
        let quiz = parse(source).expect("quiz must validate");
        let formatted = quiz.format_source();
        let reparsed = parse(&formatted).expect("escaped quiz must validate");

        assert_eq!(reparsed.questions[0].question, "A \"quoted\" line\\path");
        assert_eq!(reparsed.questions[0].options[0], "first\nline");
        assert_eq!(reparsed.answers[0].answer, "first\nline");
    }

    #[test]
    fn parses_optional_explanation_and_formats_canonically() {
        let source = r#"
            [quiczk]
            q1 = "Question 1"
            o1 = ["opt1", "opt2"]
            a1 = "opt1"
            e1 = "Because opt1 is correct"
            q2 = "Question 2"
            o2 = ["optA", "optB"]
            a2 = "optA"
            "#;
        let quiz = parse(source).expect("quiz with explanation must validate");

        assert_eq!(
            quiz.questions[0].explanation.as_deref(),
            Some("Because opt1 is correct")
        );
        assert_eq!(quiz.questions[1].explanation, None);

        let expected = concat!(
            "[quiczk]\n",
            "q1 = \"Question 1\"\n",
            "o1 = [\n",
            "  \"opt1\",\n",
            "  \"opt2\",\n",
            "]\n",
            "a1 = \"opt1\"\n",
            "e1 = \"Because opt1 is correct\"\n\n",
            "q2 = \"Question 2\"\n",
            "o2 = [\n",
            "  \"optA\",\n",
            "  \"optB\",\n",
            "]\n",
            "a2 = \"optA\"\n",
        );
        assert_eq!(quiz.format_source(), expected);
    }

    #[test]
    fn rejects_orphaned_explanation_keys() {
        let source = r#"
            [quiczk]
            q1 = "Question 1"
            o1 = ["opt1", "opt2"]
            a1 = "opt1"
            e2 = "Orphaned explanation"
            "#;
        let error = parse(source).expect_err("orphaned explanation key must fail");

        assert!(
            error
                .to_string()
                .contains("e2 is provided without matching q2")
        );
    }
}
