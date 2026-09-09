# quiczk

Quiz yourself in TUI.

<p align="center">
  <img src="vhs.gif" alt="quiczk TUI demo" />
</p>

## TOML parsing format

```toml
[config]
shuffle_questions = false
shuffle_options = false
random_start_cursor = false
hide_options_until_interact = true

[quiczk]
q1 = "Who is the author of quiczk?"
o1 = [
  "Linus Benedict Torvalds",
  "Peerawat Monviset",
  "Augusta Ada King, Countess of Lovelace",
  "Alan Mathison Turing",
]
a1 = "Peerawat Monviset"

q2 = "How do you exit Vim?"
o2 = [
  ":wq",
  "buy a new laptop",
  "pull the power plug",
  "turn off the breaker",
]
a2 = ":wq"
e2 = "':wq' writes changes and quits the editor."
```

## Config

`$XDG_CONFIG_HOME/quiczk/config.toml`, or `~/.config/quiczk/config.toml`:

```toml
[config]
shuffle_questions = false
shuffle_options = true
random_start_cursor = true
hide_options_until_interact = true
question_timer_seconds = 0
```

A quiz file can embed its own `[config]` table for per-quiz recommendations.
Embedded keys take precedence over the XDG config file, which takes precedence
over compiled defaults. Omitted keys fall through to the next layer, and an
omitted or `0` timer disables the countdown:

```toml
[config]
question_timer_seconds = 30
```
