# Contributing to Dotlanth

Thank you for your interest in contributing. This guide covers the workflow, expectations, and required tooling.

## Issue Workflow

1. **Find or create an issue** with appropriate labels:
   - `phase:0-governance`, `phase:1-core`, etc. (development phase)
   - `type:documentation`, `type:feature`, `type:bug` (work type)
   - `milestone:26.1.0-alpha.1` (target release)
2. **Comment on the issue** to signal you are working on it
3. **Submit a pull request** referencing the issue (e.g., `Fixes #1`)
4. **Wait for review** from a maintainer

## Commit Hygiene

- Use clear, descriptive commit messages
- Keep commits focused on a single logical change
- Follow the [Conventional Commits](https://www.conventionalcommits.org/) style when possible (e.g., `docs: add contributing guidelines`)

## Review Expectations

- All PRs require at least one maintainer approval
- Address review feedback promptly
- Ensure all checks pass before requesting re-review

## Required Commands

Before submitting a pull request, run these commands locally:

```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy -- -D warnings

# Run tests
cargo test
```

All three must pass without errors or warnings before submission.
