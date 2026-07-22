# Changelog

All notable changes to this project will be documented in this file.

## [0.3.1]

### Added
- `bb env create <name> [--env-type Test|Staging|Production]` — create a
  deployment environment (requires an `admin:pipeline:bitbucket` token).
  Resolves a long-standing gap from the original project (issue bb-cli/bb-cli#24).

### Changed
- Test coverage: the HTTP client's auth/error ladder is now covered by
  `httpmock` integration tests (Basic auth header, 401/409/type=error/status).

## [0.3.0]

### Added
- **MCP server** (`bb mcp serve`) — exposes Bitbucket to AI assistants over the
  Model Context Protocol (stdio). Tools: read-only (`pr_list`, `pr_diff`,
  `pipeline_latest`, `branch_list`), a generic `bitbucket_api` passthrough, and
  mutating tools (`pr_approve`, `pr_merge`, `pipeline_run`) flagged DESTRUCTIVE
  so clients confirm before acting. Reuses the CLI's authenticated client.

### Changed
- `bb api` gained `--paginate` (follow `next` links, emit all pages as one
  array); install.sh falls back to `~/.local/bin` robustly; CI actions bumped
  (checkout v7, action-gh-release v3).

## [0.2.0]

### Added — coverage toward `gh` parity
- `bb api <endpoint>` — authenticated passthrough to any Bitbucket REST
  endpoint, with `{repo}`/`{workspace}` placeholders, `-f key=value` body
  fields, and `--input`.
- `bb repo` — list/view/create/clone/fork/delete.
- `bb issue` — list/view/create/comment/close.
- `bb alias` — local command shortcuts with `$1..$N` expansion.
- `bb snippet` — list/view. `bb webhook` — list/create/delete.
  `bb key` — SSH keys list/add/delete.
- `bb search repos` — repository search within a workspace.
- `bb workspace` — list workspaces and their projects.
- `bb extension` — plugin system: `bb <name>` runs a `bb-<name>` executable
  from `~/.bb/extensions`; install/list/remove.

## [0.1.0]

### Changed
- **Rewrite from PHP to Rust.** `bb` is now a single static binary with no
  runtime dependency, distributed as prebuilt per-platform binaries.
- **Authentication migrated to Atlassian API tokens** (Basic `email:apiToken`),
  replacing the removed Bitbucket app passwords. Legacy app-password configs are
  detected and prompt re-authentication.
- Release pipeline now cross-compiles macOS (arm64/x86_64) and Linux
  (x86_64/aarch64) binaries; `bb upgrade` selects the matching asset. Docker
  image and PHAR build removed.

### Added
- `bb pr show` command for viewing PR comments with inline code comment support.
  Usage: `bb pr show <pr_id> [unresolved]`

---

## PHP releases (original bb-cli)

The entries below are from the original PHP project this tool was ported from.

---

## [1.0.2] - 2024-02-21
### Add
- run command for pipeline
    - For more info see: [Link](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pipelines/#api-repositories-workspace-repo-slug-pipelines-post)

## [1.0.1] - 2023-02-16
### Fix
- upgrade command folder check fix

## [1.0.0] - 2023-02-16
### Add
- upgrade command

## [0.3.0] - 2022-10-07
### Fix
- fix missing extension list

## [0.2.0] - 2022-10-07
### Add
- add version argument

## [0.1.0] - 2022-09-27
- Initial release
