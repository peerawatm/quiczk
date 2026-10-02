use std::fs;

#[test]
fn readme_contains_example_toml() {
    let readme = fs::read_to_string("README.md").expect("read README.md");
    let example = fs::read_to_string("example.toml").expect("read example.toml");
    let example = example.trim();
    assert!(
        readme.contains(example),
        "README.md must contain exact example.toml verbatim; run `just fmt-readme` or copy example.toml into README.md"
    );
}
