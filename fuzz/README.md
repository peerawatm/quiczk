# Fuzzing

Install `cargo-fuzz` and use a nightly toolchain:

```sh
cargo install cargo-fuzz
cargo +nightly fuzz run quiz_source -- -max_total_time=60
```

The `quiz_source` target feeds arbitrary UTF-8 TOML source into the quiz
parser and validator without opening files or initializing the terminal.
