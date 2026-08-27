# quiczk

Quiz yourself in TUI.

## TOML parsing format

```toml
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
question_timer_seconds = 15
```
