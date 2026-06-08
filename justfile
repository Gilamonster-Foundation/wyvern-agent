# wyvern-agent — task runner
#
# PIPELINE PARITY: this justfile is the local equivalent of CI. When a
# CI/CD pipeline is added under .github/workflows/, keep its lint /
# format / test steps in sync with `just check` and update the pre-push
# hook (.githooks/pre-push) to match (see CLAUDE.md "Push Hook
# Governance").

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
    cargo clippy --workspace --all-targets -- -D warnings

# Run the whole test suite.
test:
    cargo test --workspace

# HARD RULE: no vendor identities in non-test source. Workers are
# selected by capability (agent-bridle Caveats), never by provider. The
# authoritative machine-checked version is the test
# `no_vendor_identities_in_src` in wyvern-wire.
no-vendor:
    cargo test -p wyvern-wire --test no_vendor

# Install the pre-push hook (mirrors `just check`).
install-hooks:
    git config core.hooksPath .githooks
