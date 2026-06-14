# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Open-source project files: `LICENSE`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `SECURITY.md`, `CHANGELOG.md`, and GitHub issue/PR templates.

### Changed
- `README.md` rewritten in English, with a practical **Quick start** section
  (install → `ado-cli skill` → configure → run Claude).

## [0.1.3]

### Added
- `ado-cli skill` installs the Claude Code skill and configures credentials
  (`.env`) interactively.
- Read commands: `query`, `get`, `links`, `list-comments`,
  `list-work-item-types`, `list-iterations`, `current-sprint`, `my-work-items`,
  `taskboard`, `taskboard-columns`, `list-team-members`, `search-users`.
- Write commands: `create`, `create-child-tasks`, `update`, `assign`,
  `add-comment`, `set-backlog-priority`, `add-link`, `add-tags`, `remove-tags`,
  `move-to-iteration`, `move-to-current-sprint`, `move-to-backlog`, `set-state`,
  `set-taskboard-column`.
- npm distribution (`@danielfalci/ado-cli`) via cargo-dist; prebuilt binaries for
  macOS (arm64/x64), Linux (arm64/x64) and Windows x64.

[Unreleased]: https://github.com/dfalci/ado-cli/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/dfalci/ado-cli/releases/tag/v0.1.3
