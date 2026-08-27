#![no_main]

use libfuzzer_sys::fuzz_target;
use quiczk::quiz::Quiz;
use std::path::Path;

fuzz_target!(|input: &[u8]| {
    let Ok(source) = std::str::from_utf8(input) else {
        return;
    };

    let _ = Quiz::from_source(Path::new("fuzz.toml"), source);
});
