use std::{
    fmt::Write as _,
    io,
    time::{Duration, Instant},
};

use rand::{RngExt, rngs::ThreadRng, seq::SliceRandom};
use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
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
    config: Config,
    question_started_at: Instant,
    timed_out: bool,
    options_revealed: bool,
    rng: Option<ThreadRng>,
}

struct RuntimeQuestion {
    prompt: String,
    options: Vec<String>,
    answer: String,
    explanation: Option<String>,
}

impl App {
    pub fn new(quiz: Quiz, mut config: Config, title: String) -> Self {
        config.question_timer_seconds = config.question_timer_seconds.filter(|&s| s > 0);
        let hide = config.hide_options_until_interact;

        let mut questions = quiz
            .questions
            .into_iter()
            .zip(quiz.answers)
            .map(|q| RuntimeQuestion {
                prompt: q.0.question,
                options: q.0.options,
                answer: q.1.answer,
                explanation: q.0.explanation,
            })
            .collect::<Vec<_>>();

        // RNG is created only when shuffle or random cursor needs it,
        // avoiding a getrandom syscall plus ChaCha seeding otherwise.
        let mut rng: Option<ThreadRng> = None;
        if config.shuffle_questions {
            questions.shuffle(rng.get_or_insert_with(rand::rng));
        }
        if config.shuffle_options {
            let rng = rng.get_or_insert_with(rand::rng);
            for q in &mut questions {
                q.options.shuffle(&mut *rng);
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
            config,
            question_started_at: Instant::now(),
            timed_out: false,
            options_revealed: !hide,
            rng,
        };
        app.selected_option = app.next_start_index();
        app
    }

    fn current(&self) -> &RuntimeQuestion {
        &self.questions[self.question_index]
    }

    fn next_start_index(&mut self) -> usize {
        let len = self.questions[self.question_index].options.len();
        if self.config.random_start_cursor {
            self.rng.get_or_insert_with(rand::rng).random_range(0..len)
        } else {
            0
        }
    }

    fn timer_running(&self) -> bool {
        self.config.question_timer_seconds.is_some() && !self.answered && self.options_revealed
    }

    fn remaining_seconds(&self) -> Option<u64> {
        let total = self.config.question_timer_seconds?;
        if self.answered || !self.options_revealed {
            return Some(total);
        }
        let elapsed = self.question_started_at.elapsed().as_secs();
        Some(total.saturating_sub(elapsed))
    }

    /// Single-elapsed timer tick. Returns remaining on active countdown.
    fn poll_timer(&mut self) -> Option<u64> {
        let total = self.config.question_timer_seconds?;
        if self.answered || !self.options_revealed {
            return None;
        }
        let elapsed = self.question_started_at.elapsed();
        if elapsed >= Duration::from_secs(total) {
            self.answered = true;
            self.timed_out = true;
            self.question_results[self.question_index] = Some(false);
            return Some(total);
        }
        Some(total.saturating_sub(elapsed.as_secs()))
    }

    fn restart(&mut self) {
        if self.config.shuffle_questions {
            let rng = self.rng.get_or_insert_with(rand::rng);
            self.questions.shuffle(&mut *rng);
        }
        if self.config.shuffle_options {
            let rng = self.rng.get_or_insert_with(rand::rng);
            for q in &mut self.questions {
                q.options.shuffle(&mut *rng);
            }
        }
        self.question_index = 0;
        self.selected_option = self.next_start_index();
        self.answered = false;
        self.score = 0;
        self.question_results.fill(None);
        self.question_started_at = Instant::now();
        self.timed_out = false;
        self.options_revealed = !self.config.hide_options_until_interact;
    }

    fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            _ => {}
        }
        if !self.options_revealed && !self.answered {
            self.options_revealed = true;
            self.question_started_at = Instant::now();
            return false;
        }
        match key {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('w') if !self.answered => {
                self.selected_option = self.selected_option.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('s') if !self.answered => {
                self.selected_option =
                    (self.selected_option + 1).min(self.current().options.len() - 1);
            }
            KeyCode::Enter if !self.answered => {
                self.answered = true;
                let correct = self.current().options[self.selected_option] == self.current().answer;
                self.question_results[self.question_index] = Some(correct);
                if correct {
                    self.score += 1;
                }
            }
            KeyCode::Enter => {
                if self.question_index + 1 < self.questions.len() {
                    self.question_index += 1;
                    self.selected_option = self.next_start_index();
                    self.answered = false;
                    self.question_started_at = Instant::now();
                    self.timed_out = false;
                    self.options_revealed = !self.config.hide_options_until_interact;
                } else {
                    self.restart();
                }
            }
            _ => {}
        }
        false
    }
}

/// Restores the terminal on early error returns from the event loop.
struct RestoreGuard {
    armed: bool,
}

impl Drop for RestoreGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = ratatui::try_restore();
        }
    }
}

pub fn run(mut app: App) -> io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let _restore = RestoreGuard { armed: true };
    let mut remaining = app.remaining_seconds();
    terminal.draw(|frame| draw(frame, &app, remaining))?;

    let mut last_remaining = remaining;
    let result = loop {
        let running = app.timer_running();
        // Block when idle so quiczk sleeps at 0% CPU. Wake on next
        // second boundary only while countdown is active.
        let timeout = if running {
            let ms_into_sec = app.question_started_at.elapsed().subsec_millis();
            Duration::from_millis(1000 - u64::from(ms_into_sec.min(999)))
        } else {
            Duration::from_secs(3600)
        };
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    // Snapshot visible state so no-op keys skip redraw.
                    let before = (
                        app.question_index,
                        app.selected_option,
                        app.answered,
                        app.timed_out,
                        app.options_revealed,
                        app.score,
                    );
                    if app.handle_key(key.code, key.modifiers) {
                        break Ok(());
                    }
                    let after = (
                        app.question_index,
                        app.selected_option,
                        app.answered,
                        app.timed_out,
                        app.options_revealed,
                        app.score,
                    );
                    if after != before {
                        remaining = app.remaining_seconds();
                        last_remaining = remaining;
                        terminal.draw(|frame| draw(frame, &app, remaining))?;
                    }
                }
                Event::Resize(..) => {
                    terminal.draw(|frame| draw(frame, &app, remaining))?;
                }
                _ => {}
            }
        } else if running {
            let was_answered = app.answered;
            if let Some(now) = app.poll_timer() {
                if Some(now) != last_remaining || app.answered != was_answered {
                    last_remaining = Some(now);
                    remaining = Some(now);
                    terminal.draw(|frame| draw(frame, &app, remaining))?;
                }
            }
        }
    };

    ratatui::try_restore()?;
    std::mem::forget(_restore);
    result
}

fn draw(frame: &mut Frame, app: &App, remaining: Option<u64>) {
    let current = app.current();
    let mut lines = progress_lines(app, frame.area().width, remaining);
    lines.reserve(8 + current.options.len());
    lines.extend([
        Line::default(),
        Line::from(format!("{}.", app.question_index + 1)),
        Line::from(current.prompt.as_str()),
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
                Span::styled(option.as_str(), style),
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
                Span::styled(explanation.as_str(), Style::default().fg(Color::Reset)),
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

fn progress_lines(app: &App, width: u16, remaining: Option<u64>) -> Vec<Line<'static>> {
    let label = if app.questions.len() == 1 {
        "question"
    } else {
        "questions"
    };
    let mut header = String::with_capacity(app.title.len() + 32);
    header.push_str(app.title.as_str());
    let _ = write!(header, " | {} {}", app.questions.len(), label);
    if let Some(s) = remaining {
        let _ = write!(header, " | [{s}s]");
    }
    header.push(' ');
    let mut lines = vec![Line::from(std::mem::take(&mut header))];
    lines.reserve(2);
    lines[0]
        .spans
        .reserve(app.question_results.len().saturating_mul(2));
    let width = usize::from(width.max(1));
    let prefix_width = lines[0].width();
    let mut line_width = prefix_width;
    let mut markers_on_line = 0;

    for (index, result) in app.question_results.iter().enumerate() {
        let style = match result {
            Some(true) => Style::default().fg(Color::Green),
            Some(false) => Style::default().fg(Color::Red),
            None if index == app.question_index => Style::default().fg(Color::Reset),
            None => Style::default().fg(Color::DarkGray),
        };
        // Marker width is constant, track line width instead of rescanning.
        const MARKER_WIDTH: usize = 1;
        let should_wrap = if markers_on_line == 0 {
            lines.len() == 1 && line_width + MARKER_WIDTH > width
        } else {
            line_width + 1 + MARKER_WIDTH > width
        };
        if should_wrap {
            lines.push(Line::raw(" ".repeat(prefix_width)));
            line_width = prefix_width;
            markers_on_line = 0;
        }
        let line = lines.last_mut().expect("progress always has a line");
        if markers_on_line > 0 {
            line.spans.push(Span::raw(" "));
            line_width += 1;
        }
        line.spans.push(Span::styled("■", style));
        line_width += MARKER_WIDTH;
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
            config: None,
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
