# Contributing to ado-cli

Thanks for your interest in improving **ado-cli**! This document explains how to
set up the project, the quality bar, and how to submit changes.

## Ground rules

- Be respectful — see the [Code of Conduct](CODE_OF_CONDUCT.md).
- Open an issue before a large change, so we can agree on the approach.
- Keep pull requests focused: one logical change per PR.
- Never commit secrets. The `.env` (which holds your `AZDO_PAT`) is git-ignored —
  keep it that way. See [SECURITY.md](SECURITY.md).

## Development setup

You need a recent stable [Rust toolchain](https://rustup.rs/).

```bash
git clone https://github.com/dfalci/ado-cli
cd ado-cli
cargo build
```

To run the CLI against a real board, configure credentials first (see the
[README](README.md#configuration)):

```bash
ado-cli skill   # interactive: writes .claude/skills/azure-devops-tasks/.env
```

## Quality bar (run before pushing)

CI runs exactly these — they must pass:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

- **Formatting:** `cargo fmt --all` (no manual style debates).
- **Lints:** clippy must be clean with warnings treated as errors.
- **Tests:** add or update unit tests for any behavior you change. Network calls
  are not exercised in tests — keep logic testable by separating pure functions
  (parsing, URL building, projections) from the HTTP layer, as the existing code
  does.

## Commit messages

Use clear, imperative messages (e.g. `add --fields support to get`). Conventional
Commits are welcome but not required.

## Pull requests

1. Fork and branch from `main` (e.g. `feat/get-fields`, `fix/project-typo-hint`).
2. Make your change with tests and docs (update `README.md` and, when relevant,
   the skill at `src/skill.rs` / `SKILL.md`).
3. Ensure the full quality bar passes locally.
4. Open the PR using the template; describe the motivation and any user-facing
   change.

## Releasing (maintainers)

Releases are driven by the version in `Cargo.toml` and `cargo-dist`. See the
**CI / Release** section of the [README](README.md#ci--release).

## License

By contributing, you agree that your contributions will be licensed under the
[MIT License](LICENSE).
