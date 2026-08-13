# http-date development recipes.
# Requires `just`, and `nix` for the fuzzing recipes.

# List available recipes.
default:
    @just --list

# Run the test suite with cargo-nextest (included in the nix dev shell).
test:
    cargo nextest run --all-targets --all-features

# Type-check the crate (lib, integration tests, and doctests).
check:
    cargo check --all-targets --all-features

# Lint with clippy; the crate enables `all` and `pedantic` lints, and this
# recipe fails on any warning so it is CI-friendly.
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format the code with rustfmt.
fmt:
    cargo fmt

# Verify the code is formatted without modifying it (for CI).
fmt-check:
    cargo fmt --check

# Build the API documentation.
doc:
    cargo doc --no-deps --all-features

# Build and open the API documentation in a browser.
doc-open:
    cargo doc --no-deps --all-features --open

# ---------------------------------------------------------------------------
# Fuzzing with cargo-fuzz.
#
# The flake provides a dedicated `fuzz` dev shell with the nightly toolchain
# (required by libFuzzer's sanitizers), cargo-fuzz, and clang. These recipes
# enter it automatically, so they work from any shell.
#
# Targets: `decode` (decode never panics) and `roundtrip` (decode/encode
# round-trip property).
# ---------------------------------------------------------------------------

# List the available fuzz targets.
fuzz-list:
    nix develop .#fuzz --command cargo fuzz list

# Build a fuzz target without running it.
fuzz-build TARGET='roundtrip':
    nix develop .#fuzz --command cargo fuzz build {{TARGET}}

# Run a fuzz target until it finds a crash or is interrupted.
# Cap the iterations with RUNS, e.g. `just fuzz roundtrip 2000`.
fuzz TARGET='roundtrip' RUNS='':
    nix develop .#fuzz --command cargo fuzz run {{TARGET}} -- {{ if RUNS != '' { '-runs=' + RUNS } else { '' } }}

# Remove generated fuzz corpora and crash artifacts.
fuzz-clean:
    rm -rf fuzz/corpus fuzz/artifacts

# Run all CI checks: format, lint, and tests.
ci: fmt-check lint test
