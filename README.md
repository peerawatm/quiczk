# quiczk

Terminal quiz runtime; reads and validates one TOML file.

## Quiz

```toml
[quiczk]
q1 = "What is the default shell on macOS?"
o1 = ["bash", "zsh", "fish", "tcsh"]
a1 = "zsh"

q2 = "What command lists files?"
o2 = ["ls", "cd", "pwd", "mv"]
a2 = "ls"
```

`qN`, `oN`, and `aN` must match; numbering starts at `1` and is contiguous; `aN` must exactly match one item in `oN`. `eN` is an optional explanation string displayed after answering.

## Run

```sh
cargo run -- example.toml
# or, after cargo install --path .:
quiczk example.toml

# Format quiz files in place
quiczk fmt example.toml
quiczk fmt 2.toml example.toml
```

`quiczk fmt` groups each question, options, answer, and optional explanation, and writes the canonical multiline format directly to each file.

## Config

`$XDG_CONFIG_HOME/quiczk/config.toml`, or `~/.config/quiczk/config.toml`:

```toml
[config]
shuffle_questions = false
shuffle_options = true
random_start_cursor = true
question_timer_seconds = 15
```

`random_start_cursor` chooses a random option as the initial cursor position for each question and restart; set it to `false` to always start at the first option.
`question_timer_seconds` sets an optional countdown timer deadline in seconds per question; omit or set to `0` to disable.

## Controls

`j`/`s`/`Down` · `k`/`w`/`Up` · `Enter` · `q`/`Esc`/`Ctrl-C`

## LLM

```text
Output valid TOML only for quiczk.
Use exactly one [quiczk] table and contiguous qN/oN/aN keys starting at 1 with optional eN explanation strings.
Each oN must contain unique options; each aN must exactly match an item in oN.
Do not output [[quiczk.questions]], YAML, JSON, Markdown, or comments.
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Tagged releases publish platform archives through GitHub Actions.
