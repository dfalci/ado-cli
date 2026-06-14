# Security Policy

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Report vulnerabilities privately via
[GitHub Security Advisories](https://github.com/dfalci/ado-cli/security/advisories/new),
or by email to **danielfalci@gmail.com**.

Include, when possible:
- a description of the issue and its impact,
- steps to reproduce,
- the affected version (`ado-cli --version`).

You can expect an initial acknowledgement within a few business days.

## Supported versions

This is a small, fast-moving tool. Security fixes are applied to the **latest**
released version on npm / GitHub Releases. Please upgrade before reporting.

## Handling credentials (important for users)

`ado-cli` authenticates to Azure DevOps with a **Personal Access Token (PAT)**,
which grants real write access to your board. To keep it safe:

- The PAT lives in `.claude/skills/azure-devops-tasks/.env`. This path is
  **git-ignored** by default — never commit it, and never paste a PAT into an
  issue, PR, log, or screenshot.
- Scope the PAT minimally: **Work Items (Read & Write)** is enough for most
  commands; `search-users` additionally needs **Identity (Read)**.
- Rotate the PAT periodically and revoke it immediately if it may have leaked
  (Azure DevOps → User settings → Personal access tokens).
- The CLI prints API responses as JSON to stdout; be mindful of where you redirect
  that output, as it can contain work-item content.
