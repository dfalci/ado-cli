# ado-cli

[![npm](https://img.shields.io/npm/v/@danielfalci/ado-cli)](https://www.npmjs.com/package/@danielfalci/ado-cli)
[![ci](https://github.com/dfalci/ado-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/dfalci/ado-cli/actions/workflows/ci.yml)

A local (Rust) CLI that exposes the **tasks (work items) of an Azure DevOps
board** from the command line: list/query, create, update, change state, assign,
move taskboard columns, link items (backlog hierarchy), reorder the backlog,
navigate sprint context, decompose items into sub-tasks, and manage tags and
comments.

Every operation prints **JSON** to stdout. It is the command-line counterpart of
the `mcp-ado` MCP server (which still exists, independently).

## Quick start

End-to-end, from zero to driving your board with Claude:

```bash
# 1. Install the CLI globally (pulls the native binary for your platform)
npm install -g @danielfalci/ado-cli

# 2. Go into the project where you want to use it
cd /path/to/your/project

# 3. Install the skill + configure credentials (interactive)
ado-cli skill
#   → it writes .claude/skills/azure-devops-tasks/SKILL.md
#   → then it prompts you, interactively:
#       AZDO_PAT      : <paste your Personal Access Token>
#       AZDO_PROJECT  : organization/project   (e.g. contoso/Store)
#       AZDO_TEAM     : <Enter to accept the default "{project} Team">
#   → and saves .claude/skills/azure-devops-tasks/.env for you

# 4. Sanity-check the connection (read-only)
ado-cli my-work-items

# 5. Run Claude in that folder — the skill is now active
claude
```

That's it. From inside Claude you can now say things like *"show my tasks"*,
*"tell me about #2287"*, or *"create a user story for X"*, and the agent will use
`ado-cli` following the safety rules baked into the skill (read freely; write only
on an explicit order).

> **Heads up on `AZDO_PROJECT`:** the value must match an existing project
> **exactly** (e.g. `contoso/Store`). A typo produces
> `TF200016 ... project does not exist`. To list the valid projects in your org:
> `curl -u :$AZDO_PAT https://dev.azure.com/<org>/_apis/projects?api-version=7.1`.

## Installation

Via npm (installs the native binary for your platform automatically):

```bash
npm install -g @danielfalci/ado-cli
```

Supported platforms: macOS (arm64/x64), Linux (arm64/x64) and Windows x64. There
are also prebuilt binaries in each
[GitHub Release](https://github.com/dfalci/ado-cli/releases). Or build from source
(see **Build**).

## Build

```bash
cargo build --release
# binary at target/release/ado-cli
```

## Configuration

Configuration comes from a **`.env` file in the skill folder**
(`.claude/skills/azure-devops-tasks/.env`, relative to the current directory), in
`KEY=value` format. For each key missing from the file, it falls back to the OS
environment variable of the same name. **There are no configuration flags in the
CLI** — the command line only takes operation arguments.

The recommended way to configure is to run **`ado-cli skill`**: in an interactive
terminal it asks for the credentials and writes the `.env` to the correct folder
(see **Claude Code skill**).

| Variable            | Required | Default                  | Description                                   |
| ------------------- | -------- | ------------------------ | --------------------------------------------- |
| `AZDO_PAT`          | yes      | —                        | Personal Access Token (Work Items r/w scope). |
| `AZDO_PROJECT`      | yes      | —                        | In the `organization/project` format.         |
| `AZDO_TEAM`         | no       | `{project} Team`         | Team used by the sprint/iteration APIs.       |
| `AZDO_BASE_URL`     | no       | `https://dev.azure.com`  | Useful for on-prem Azure DevOps Server.        |
| `AZDO_API_VERSION`  | no       | `7.1`                    | REST API version.                             |

Example `.claude/skills/azure-devops-tasks/.env`:

```
AZDO_PAT=<your-pat>
AZDO_PROJECT=contoso/Store
```

## Usage

```bash
ado-cli <command> [args]
# output is always JSON on stdout

ado-cli --help          # list all commands
ado-cli <command> --help
```

### Examples

```bash
# Read
ado-cli query                               # no WIQL: only the most recent OPEN ones
ado-cli query --include-closed              # no WIQL: include closed ones
ado-cli query --wiql "SELECT [System.Id] FROM WorkItems WHERE [System.WorkItemType]='Bug' AND [System.State]='Active'"
ado-cli get 123
ado-cli links 10
ado-cli current-sprint --fields System.Id,System.Title,System.State
ado-cli taskboard
ado-cli my-work-items                       # by default, only open ones
ado-cli my-work-items --include-closed      # include closed ones
ado-cli my-work-items --only-current-sprint

# Write
ado-cli create --type Bug --title "Checkout error" --repro-steps "1. ..." --priority 1
ado-cli update 123 --set System.Title="New title" --set Microsoft.VSTS.Common.Priority=2
ado-cli update 123 --json '{"System.Title":"New title","Microsoft.VSTS.Common.Priority":2}'
ado-cli assign 123 "someone@company.com"
ado-cli add-link 10 42 --link-type child
ado-cli add-comment 123 "A comment"

# Write 🔴 (state/column): only on an explicit order
ado-cli set-state 77 Closed
ado-cli set-taskboard-column 123 --column "Em Desenvolvimento"

# Decompose a parent into sub-tasks (JSON array via --json or stdin)
ado-cli create-child-tasks --parent-id 10 --json '[{"title":"Implement API"},{"title":"Test"}]'
echo '[{"title":"A"},{"title":"B"}]' | ado-cli create-child-tasks --parent-id 10
```

Commands with complex structure (`update`, `create-child-tasks`) accept **JSON**
via flag (`--json`) or via **stdin** when the flag is omitted.

## Claude Code skill

The binary installs a skill that teaches the agent how to use this CLI and, in the
same step, configures the credentials:

```bash
cd /your/project
ado-cli skill
```

What `ado-cli skill` does:

- writes `./.claude/skills/azure-devops-tasks/SKILL.md` (overwrites if present);
- writes a template `.env.example` in the same folder;
- in an **interactive terminal**, prompts for the credentials (PAT, project, and
  the optionals) and writes the **`.env` directly into the correct folder** —
  asking for confirmation before overwriting an existing `.env`. Outside a
  terminal, it just indicates where to create the `.env`.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

## CI / Release

The release is done with [cargo-dist](https://axodotdev.github.io/cargo-dist)
(`dist`), configured in `dist-workspace.toml`.

- **`.github/workflows/ci.yml`** — on every push to `main` and on PRs: runs
  `fmt --check`, `clippy -D warnings`, `cargo test`, and `cargo build --release`.
- **`.github/workflows/release.yml`** (generated by cargo-dist) — triggered by a
  **version tag** (`vX.Y.Z`): builds the binaries for macOS (arm64/x64), Linux
  (arm64/x64) and Windows x64, creates the **GitHub Release** with the files, and
  generates the **npm** installer (`ado-cli-npm-package.tar.gz`).
- **`.github/workflows/publish-npm.workflow-run.yml`** — runs after `Release`
  finishes: downloads the `*-npm-package.tar.gz` from the Release and runs
  `npm publish` (`@danielfalci/ado-cli`). Requires the **`NPM_TOKEN`** secret.

**The version is the one in `Cargo.toml`** — it defines the binary version
(`ado-cli --version`/`--help`), the tag, the GitHub Release, and the npm package,
all matching. The release flow:

1. Update `version` in `Cargo.toml` (e.g. `0.2.0`).
2. Run `publish.bat` (reads the version from `Cargo.toml`, creates and pushes the
   `vX.Y.Z` tag).

The tag triggers `release.yml`; once it finishes, `publish-npm` publishes to npm.
The npm installer (`ado-cli-npm-package.tar.gz`) is generated by cargo-dist itself
— there is no npm packaging code versioned in the repo. To regenerate the CI after
changing `dist-workspace.toml`: `dist generate`.

## License

MIT.
