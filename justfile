# Development tasks for rcp2-cli. Run `just` to see all recipes.

# List available recipes
default:
    @just --list

# Run the CLI with arbitrary arguments, e.g. `just run connect --full`
run *ARGS:
    cargo run -p rcp2-cli -- {{ ARGS }}

# Launch the TUI
tui *ARGS:
    cargo run -p rcp2-cli -- tui {{ ARGS }}

# Launch the TUI in dry-run mode, writing the write-log to trace.log
trace *ARGS:
    cargo run -p rcp2-cli -- tui --dry-run {{ ARGS }} 2> trace.log

# Debug build of the whole workspace
build:
    cargo build --workspace --all-targets

# Optimized release build
release:
    cargo build --workspace --release

# Run all workspace tests
test *ARGS:
    cargo test --workspace {{ ARGS }}

# Clippy with warnings as errors (same lints as CI)
lint:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Clippy with pedantic lints on top, for local review only
lintp:
    cargo clippy --workspace --all-targets --all-features --locked -- -W clippy::pedantic

# Apply rustfmt to the workspace
fmt:
    cargo fmt --all

# Check formatting without writing (CI-style)
fmt-check:
    cargo fmt --all --check

# Check licenses, advisories, bans and sources
deny:
    cargo deny check

# Audit the GitHub workflows with zizmor (same settings as CI)
zizmor:
    zizmor --persona pedantic --no-online-audits .github/workflows/

# Update dependencies in Cargo.lock
update:
    cargo update

# Everything CI checks, in the order that fails fastest
ci: fmt-check lint test deny

# Install rcp2-cli into ~/.cargo/bin
install:
    cargo install --path crates/rcp2-cli --locked

# Install the udev rules and reload them (requires sudo)
udev:
    sudo install -m 644 udev/50-rodecaster.rules /etc/udev/rules.d/50-rodecaster.rules
    sudo udevadm control --reload-rules
    sudo udevadm trigger

# Remove build artifacts
clean:
    cargo clean
