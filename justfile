default: run

run:
	cargo run -- example.toml

dev:
	cargo build

fmt:
	cargo fmt --all -- --check

test:
	cargo check --workspace --all-targets --all-features
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo test --workspace --all-targets --all-features

release: test
	cargo build --workspace --release

# PE + ELF + Mach-O, both archs — mirrors .github/workflows/release.yml
dist:
	cargo build --release
	cargo zigbuild --target x86_64-unknown-linux-gnu --release
	cargo zigbuild --target aarch64-unknown-linux-gnu --release
	cargo zigbuild --target x86_64-unknown-linux-musl --release
	cargo zigbuild --target aarch64-unknown-linux-musl --release
	cargo build --target x86_64-apple-darwin --release
	cargo build --target aarch64-apple-darwin --release
	-cargo xwin build --target x86_64-pc-windows-msvc --release
	-cargo xwin build --target aarch64-pc-windows-msvc --release

dist-check:
	file target/release/quiczk
	file target/x86_64-unknown-linux-gnu/release/quiczk || true
	file target/aarch64-unknown-linux-gnu/release/quiczk || true
	file target/x86_64-unknown-linux-musl/release/quiczk || true
	file target/aarch64-unknown-linux-musl/release/quiczk || true
	file target/x86_64-apple-darwin/release/quiczk || true
	file target/aarch64-apple-darwin/release/quiczk || true
	file target/x86_64-pc-windows-msvc/release/quiczk.exe || file target/x86_64-pc-windows-gnu/release/quiczk.exe || true
	file target/aarch64-pc-windows-msvc/release/quiczk.exe || file target/aarch64-pc-windows-gnu/release/quiczk.exe || true

install: release
	mkdir -p ~/.local/bin
	cp target/release/quiczk ~/.local/bin/
