use std::{
    io,
    time::{Duration, Instant},
};

use rand::{Rng, RngExt, seq::SliceRandom};
use ratatui::{
    Frame, TerminalOptions, Viewport,
    crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
        execute,
        terminal::{self, Clear, ClearType},
    },
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use crate::config::Config;
use quiczk::quiz::Quiz;

pub struct App {
    title: String,
    questions: Vec<RuntimeQuestion>,
    question_index: usize,
    selected_option: usize,
    answered: bool,
    score: usize,
    question_results: Vec<Option<bool>>,
    shuffle_questions: bool,
    shuffle_options: bool,
    random_start_cursor: bool,
    question_timer_seconds: Option<u64>,
    question_started_at: Instant,
    timed_out: bool,
    hide_options_until_interact: bool,
    options_revealed: bool,
}

struct RuntimeQuestion {
    prompt: String,
    options: Vec<String>,
    answer: String,
    explanation: Option<String>,
}

impl App {
    pub fn new(quiz: Quiz, config: Config, title: String) -> Self {
        let shuffle_questions = config.shuffle_questions;
        let shuffle_options = config.shuffle_options;
        let random_start_cursor = config.random_start_cursor;
        let question_timer_seconds = config.question_timer_seconds.filter(|&seconds| seconds > 0);
        let hide_options_until_interact = config.hide_options_until_interact;
        let mut questions = quiz
            .questions
            .into_iter()
            .zip(quiz.answers)
            .map(|question| RuntimeQuestion {
                answer: question.1.answer,
                prompt: question.0.question,
                options: question.0.options,
                explanation: question.0.explanation,
            })
            .collect::<Vec<_>>();

        let mut rng = rand::rng();
        if shuffle_questions {
            questions.shuffle(&mut rng);
        }
        if shuffle_options {
            for question in &mut questions {
                question.options.shuffle(&mut rng);
            }
        }

        let question_count = questions.len();

        let mut app = Self {
            title,
            questions,
            question_index: 0,
            selected_option: 0,
            answered: false,
            score: 0,
            question_results: vec![None; question_count],
            shuffle_questions,
            shuffle_options,
            random_start_cursor,
            question_timer_seconds,
            question_started_at: Instant::now(),
            timed_out: false,
            hide_options_until_interact,
            options_revealed: !hide_options_until_interact,
        };

        app.selected_option = app.starting_option(&mut rng);
        app
    }

    fn viewport_height(&self, width: u16) -> u16 {
        let question_height = self
            .questions
            .iter()
            .map(|question| {
                let prompt_lines = wrapped_line_count(&question.prompt, 0, width);
                let options_lines: usize = question
                    .options
                    .iter()
                    .map(|option| wrapped_line_count(option, 4, width))
                    .sum();
                let explanation_lines = question
                    .explanation
                    .as_deref()
                    .map(|explanation| wrapped_line_count(explanation, 13, width).saturating_add(1))
                    .unwrap_or(0);

                1usize
                    .saturating_add(prompt_lines)
                    .saturating_add(1)
                    .saturating_add(options_lines)
                    .saturating_add(explanation_lines)
            })
            .max()
            .unwrap_or(1)
            .saturating_add(8)
            .saturating_add(progress_lines(self, width).len());
        question_height.min(u16::MAX as usize) as u16
    }

    fn current(&self) -> &RuntimeQuestion {
        &self.questions[self.question_index]
    }

    fn starting_option(&self, rng: &mut impl Rng) -> usize {
        if self.random_start_cursor {
            rng.random_range(0..self.current().options.len())
        } else {
            0
        }
    }

    fn remaining_seconds(&self) -> Option<u64> {
        let total = self.question_timer_seconds?;
        if self.answered {
            return None;
        }
        if !self.options_revealed {
            return Some(total);
        }
        let elapsed = self.question_started_at.elapsed().as_secs();
        Some(total.saturating_sub(elapsed))
    }

    fn check_timeout(&mut self) -> bool {
        if self.answered || !self.options_revealed {
            return false;
        }
        if let Some(total) = self.question_timer_seconds
            && self.question_started_at.elapsed() >= Duration::from_secs(total)
        {
            self.answered = true;
            self.timed_out = true;
            self.question_results[self.question_index] = Some(false);
            return true;
        }
        false
    }

    fn restart(&mut self) {
        let mut rng = rand::rng();
        if self.shuffle_questions {
            self.questions.shuffle(&mut rng);
        }
        if self.shuffle_options {
            for question in &mut self.questions {
                question.options.shuffle(&mut rng);
            }
        }

        self.question_index = 0;
        self.selected_option = self.starting_option(&mut rng);
        self.answered = false;
        self.score = 0;
        self.question_results.fill(None);
        self.question_started_at = Instant::now();
        self.timed_out = false;
        self.options_revealed = !self.hide_options_until_interact;
    }

    fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => true,
            _ if !self.options_revealed && !self.answered => {
                self.options_revealed = true;
                self.question_started_at = Instant::now();
                false
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('w') if !self.answered => {
                self.selected_option = self.selected_option.saturating_sub(1);
                false
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('s') if !self.answered => {
                self.selected_option =
                    (self.selected_option + 1).min(self.current().options.len() - 1);
                false
            }
            KeyCode::Enter if !self.answered => {
                self.answered = true;
                let correct = self.current().options[self.selected_option] == self.current().answer;
                self.question_results[self.question_index] = Some(correct);
                if correct {
                    self.score += 1;
                }
                false
            }
            KeyCode::Enter => {
                if self.question_index + 1 < self.questions.len() {
                    self.question_index += 1;
                    let mut rng = rand::rng();
                    self.selected_option = self.starting_option(&mut rng);
                    self.answered = false;
                    self.question_started_at = Instant::now();
                    self.timed_out = false;
                    self.options_revealed = !self.hide_options_until_interact;
                } else {
                    self.restart();
                }
                false
            }
            _ => false,
        }
    }
}

pub fn run(mut app: App) -> io::Result<()> {
    let terminal_width = terminal::size().map(|(width, _)| width).unwrap_or(80);
    execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    let options = TerminalOptions {
        viewport: Viewport::Inline(app.viewport_height(terminal_width)),
    };
    let mut terminal = ratatui::try_init_with_options(options)?;

    terminal.draw(|frame| draw(frame, &app))?;

    let tick_rate = Duration::from_millis(100);
    let result = loop {
        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    if app.handle_key(key.code, key.modifiers) {
                        break Ok(());
                    }
                    terminal.draw(|frame| draw(frame, &app))?;
                }
                Event::Resize(..) => {
                    terminal.draw(|frame| draw(frame, &app))?;
                }
                _ => {}
            }
        } else {
            let timed_out = app.check_timeout();
            if timed_out || app.remaining_seconds().is_some() {
                terminal.draw(|frame| draw(frame, &app))?;
            }
        }
    };

    ratatui::try_restore()?;
    result
}

fn draw(frame: &mut Frame, app: &App) {
    let current = app.current();
    let mut lines = progress_lines(app, frame.area().width);
    lines.extend([
        Line::default(),
        Line::from(format!("{}.", app.question_index + 1)),
        Line::from(current.prompt.clone()),
        Line::default(),
    ]);

    if !app.options_revealed {
        lines.push(Line::from(Span::styled(
            "Press any key to reveal options.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (index, option) in current.options.iter().enumerate() {
            let (marker, style) = if !app.answered {
                if index == app.selected_option {
                    ("> ", Style::default().add_modifier(Modifier::BOLD))
                } else {
                    ("  ", Style::default())
                }
            } else if option == &current.answer {
                (
                    "✓ ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else if index == app.selected_option && !app.timed_out {
                (
                    "✗ ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(Color::DarkGray))
            };
            lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(option, style),
            ]));
        }
    }

    if let Some(correct) = app.question_results[app.question_index] {
        if let Some(explanation) = &current.explanation {
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled(
                    "Explanation: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(explanation, Style::default().fg(Color::Reset)),
            ]));
        }

        let final_question = app.question_index + 1 == app.questions.len();
        let (instruction, color) = if final_question {
            if app.timed_out {
                ("Time expired. Press Enter to restart.", Color::Red)
            } else {
                ("Press Enter to restart.", Color::Red)
            }
        } else if app.timed_out {
            (
                "Time expired. Press Enter to go to the next question.",
                Color::Red,
            )
        } else if correct {
            ("Press Enter to go to the next question.", Color::Green)
        } else {
            ("Press Enter to go to the next question.", Color::Red)
        };
        lines.push(Line::default());
        if final_question {
            let percentage = app.score.saturating_mul(100) / app.questions.len();
            lines.push(Line::from(Span::styled(
                format!(
                    "Score: {}/{} ({}%)",
                    app.score,
                    app.questions.len(),
                    percentage
                ),
                Style::default().add_modifier(Modifier::BOLD),
            )));
        }
        lines.push(Line::from(Span::styled(
            instruction,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from("Press q or ctrl-c to quit."));
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        frame.area(),
    );
}

fn wrapped_line_count(text: &str, prefix_len: usize, width: u16) -> usize {
    let width = usize::from(width.max(1));
    let mut total_lines = 0;
    for (index, line) in text.lines().enumerate() {
        let first_prefix = if index == 0 { prefix_len } else { 0 };
        let mut current_line_len = first_prefix;
        let mut lines_for_paragraph = 1;

        for word in line.split_whitespace() {
            let word_len = word.chars().count();
            if current_line_len == 0 {
                current_line_len = word_len;
            } else if current_line_len + 1 + word_len <= width {
                current_line_len += 1 + word_len;
            } else {
                lines_for_paragraph += 1;
                current_line_len = word_len;
            }
        }
        total_lines += lines_for_paragraph;
    }
    total_lines.max(1)
}

fn progress_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let timer_info = if let Some(remaining) = app.remaining_seconds() {
        format!(" | [{}s]", remaining)
    } else {
        String::new()
    };
    let mut lines = vec![Line::from(format!(
        "{} | {} {}{} ",
        app.title,
        app.questions.len(),
        if app.questions.len() == 1 {
            "question"
        } else {
            "questions"
        },
        timer_info,
    ))];
    let width = usize::from(width.max(1));
    let prefix_width = lines[0].width();
    let mut markers_on_line = 0;

    for (index, result) in app.question_results.iter().enumerate() {
        let style = match result {
            Some(true) => Style::default().fg(Color::Green),
            Some(false) => Style::default().fg(Color::Red),
            None if index == app.question_index => Style::default().fg(Color::Reset),
            None => Style::default().fg(Color::Rgb(128, 128, 128)),
        };
        let marker_width = Span::styled("■", style).width();
        let line_width = lines.last().map(Line::width).unwrap_or_default();
        let should_wrap = if markers_on_line == 0 {
            lines.len() == 1 && line_width + marker_width > width
        } else {
            line_width + 1 + marker_width > width
        };

        if should_wrap {
            lines.push(Line::raw(" ".repeat(prefix_width)));
            markers_on_line = 0;
        }

        let line = lines.last_mut().expect("progress always has a line");
        if markers_on_line > 0 {
            line.spans.push(Span::raw(" "));
        }
        line.spans.push(Span::styled("■", style));
        markers_on_line += 1;
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use quiczk::quiz::{Answer, Question};

    fn sample_quiz() -> Quiz {
        Quiz {
            questions: vec![
                Question {
                    question: "Question 1".into(),
                    options: vec!["A".into(), "B".into()],
                    explanation: None,
                },
                Question {
                    question: "Question 2".into(),
                    options: vec!["C".into(), "D".into()],
                    explanation: None,
                },
            ],
            answers: vec![Answer { answer: "A".into() }, Answer { answer: "C".into() }],
        }
    }

    #[test]
    fn options_hidden_by_default() {
        let app = App::new(sample_quiz(), Config::default(), "Test".into());
        assert!(!app.options_revealed);
    }

    #[test]
    fn options_revealed_when_disabled() {
        let config = Config {
            hide_options_until_interact: false,
            ..Config::default()
        };
        let app = App::new(sample_quiz(), config, "Test".into());
        assert!(app.options_revealed);
    }

    #[test]
    fn hide_options_until_interact_delays_reveal() {
        let config = Config {
            hide_options_until_interact: true,
            ..Config::default()
        };
        let mut app = App::new(sample_quiz(), config, "Test".into());
        assert!(!app.options_revealed);

        let quit = app.handle_key(KeyCode::Char('j'), KeyModifiers::empty());
        assert!(!quit);
        assert!(app.options_revealed);
        assert!(!app.answered);
    }

    #[test]
    fn next_question_hides_options_again_when_configured() {
        let config = Config {
            hide_options_until_interact: true,
            ..Config::default()
        };
        let mut app = App::new(sample_quiz(), config, "Test".into());

        app.handle_key(KeyCode::Char(' '), KeyModifiers::empty());
        assert!(app.options_revealed);

        app.handle_key(KeyCode::Enter, KeyModifiers::empty());
        assert!(app.answered);

        app.handle_key(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(app.question_index, 1);
        assert!(!app.options_revealed);
        assert!(!app.answered);
    }
}
