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

# macOS only: BSD install replaces the binary via fresh inode,
# avoiding stale code-signing verdicts from in-place overwrites.
install: release
	mkdir -p ~/.local/bin
	install -m 755 target/release/quiczk ~/.local/bin/

vhs:
	vhs vhs.tape
	gifsicle -O3 vhs.gif -o vhs.gif

# Bump patch version (0.0.x+1) in Cargo.toml and sync Cargo.lock.
bump:
	python3 -c 'import pathlib; p=pathlib.Path("Cargo.toml"); ls=p.read_text().splitlines(); i=next(n for n,l in enumerate(ls) if l.startswith("version = ")); M=ls[i].strip().removeprefix("version = ").strip("\"").split("."); M[2]=str(int(M[2])+1); ls[i]="version = \""+".".join(M)+"\""; p.write_text("\n".join(ls)+"\n"); print(ls[i])'
	cargo check --quiet
	@grep -m1 '^version' Cargo.toml
