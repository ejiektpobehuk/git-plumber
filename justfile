# Prefer the flake-pinned toolchain: wrap cargo in `nix develop` when Nix is
# available, unless already inside the dev shell. Opt out: `just --set wrap ""`.
have_nix := `command -v nix >/dev/null 2>&1 && echo true || echo false`
wrap     := if env("IN_NIX_SHELL", "") != "" { "" } else if have_nix == "true" { "nix develop --command " } else { "" }

# List available recipes
default:
    @just --list

# Compile the project
build:
    {{wrap}}cargo build

# Type-check without producing a binary
check:
    {{wrap}}cargo check

# Run the test suite
test:
    {{wrap}}cargo test

# Run clippy with this project's strict lint set
lint:
    {{wrap}}cargo clippy -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used

# Auto-apply clippy fixes (allows a dirty working tree)
lint-fix:
    {{wrap}}cargo clippy --allow-dirty --fix -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used

# Format Rust code
fmt:
    {{wrap}}cargo fmt

# Watch files and re-check on change (bacon)
watch:
    {{wrap}}bacon

# Build and run the TUI
run:
    {{wrap}}cargo run

# Enter the Nix dev shell
shell:
    nix develop

# Build the release package via the flake
nix-build:
    nix build

# Format Nix files with the flake's formatter
nix-fmt:
    nix fmt

# Check the flake (evaluates outputs and builds the package)
nix-check:
    nix flake check

# Update flake inputs and commit the new lock file
nix-update:
    nix flake update --commit-lock-file
