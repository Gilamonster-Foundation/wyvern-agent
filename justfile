# wyvern-agent — task runner
#
# PIPELINE PARITY: this justfile, `.github/workflows/ci.yml`, and
# `.githooks/pre-push` run the same format / lint / test / no-vendor gate.
# Audit all three when changing a gate.

# Run the full local gate: format check, lint (deny warnings), tests,
# and the no-vendor-identities guard. This is what the pre-push hook and
# CI must both run.
check: fmt-check clippy test no-vendor

# Format all crates.
fmt:
    cargo fmt --all

# Verify formatting without modifying files.
fmt-check:
    cargo fmt --all -- --check

# Lint with warnings denied (zero-warnings policy).
clippy:
    cargo clippy --locked --workspace --all-targets -- -D warnings

# Run the whole test suite.
test:
    cargo test --locked --workspace

# HARD RULE: no vendor identities in non-test source. Workers are
# selected by capability (agent-bridle Caveats), never by provider. The
# authoritative machine-checked version is the test
# `no_vendor_identities_in_src` in wyvern-wire.
no-vendor:
    cargo test --locked -p wyvern-wire --test no_vendor

# Install the pre-push hook (mirrors `just check`).
install-hooks:
    git config core.hooksPath .githooks
