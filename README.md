# bb — Bitbucket CLI

[![Latest release](https://img.shields.io/github/v/release/MarcosT96/bb-cli?sort=semver)](https://github.com/MarcosT96/bb-cli/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-000?logo=rust)](https://www.rust-lang.org)

Use Bitbucket from the command line: browse pull requests, pipelines, branches,
deployment environments and more, straight from your terminal.

![Bitbucket CLI](ss.gif)

`bb` is a single static binary written in Rust — no runtime to install, no
dependencies to manage. It authenticates with Atlassian API tokens (the
replacement for the now-removed Bitbucket app passwords).

> **Heads up:** this is a Rust rewrite of the original PHP `bb-cli`. The commands
> and config file are compatible, but authentication now uses an Atlassian API
> token **with Bitbucket scopes** instead of an app password. See
> [Acknowledgements](#acknowledgements) for the project's history.

## Installation

### 1. Install script (recommended)

One-liner that detects your OS/arch and installs the latest release binary:

```sh
curl -fsSL https://raw.githubusercontent.com/MarcosT96/bb-cli/main/install.sh | sh
```

Installs `bb` to `/usr/local/bin` (or `~/.local/bin` if that isn't writable —
make sure it's on your `PATH`). Prebuilt binaries are provided for:

- macOS (Apple Silicon & Intel)
- Linux (x86_64 & aarch64)

### 2. From source with Cargo

Requires a Rust toolchain ([rustup](https://rustup.rs)):

```sh
# From a clone
cargo install --path .

# Or straight from git
cargo install --git https://github.com/MarcosT96/bb-cli
```

### 3. Download a binary manually

Grab the asset for your platform from the
[latest release](https://github.com/MarcosT96/bb-cli/releases/latest),
named `bb-<target>` (e.g. `bb-aarch64-apple-darwin`), then:

```sh
chmod +x bb-<target>
mv bb-<target> /usr/local/bin/bb
```

Then run `bb --help`, and set up authentication with `bb auth save`
(you'll need an Atlassian API token created **with Bitbucket scopes**).

### Updating

Once installed, `bb` self-updates:

```sh
bb upgrade
```

This checks the latest GitHub release, compares versions with semver, and — if
newer — downloads and atomically replaces the running binary in place.

## Usage

Run `bb --help` (or `bb <command> --help`) to see every command and its options.
Commands include `auth`, `repo`, `pr`, `pr-details`, `issue`, `branch`,
`pipeline`, `env`, `browse`, `snippet`, `webhook`, `key`, `search`,
`workspace`, `alias`, `extension`, `upgrade`, and the generic `api` passthrough
(`bb api <endpoint>` reaches any Bitbucket REST endpoint).

## MCP server

`bb` doubles as a [Model Context Protocol](https://modelcontextprotocol.io)
server, exposing Bitbucket to AI assistants (Claude, etc.). Run it over stdio:

```sh
bb mcp serve
```

To use it with Claude Code, register it as an MCP server pointing at the `bb`
binary with `mcp serve` as its argument. The server exposes read-only tools
(`pr_list`, `pr_diff`, `pipeline_latest`, `branch_list`), a generic
`bitbucket_api` passthrough, and mutating tools (`pr_approve`, `pr_merge`,
`pipeline_run`) whose descriptions are explicitly flagged **DESTRUCTIVE** so the
assistant asks before acting. All tools reuse the same authenticated client as
the CLI, so `bb auth save` is the only setup needed.

## Roadmap

A dependency-free Rust binary that's also an MCP server is the foundation.
Where the project is headed — direction, not dated promises:

- **Deeper AI-assisted workflows.** Building on the MCP server: summarizing
  pull requests and review threads, drafting PR descriptions, triaging pipeline
  failures, and surfacing what needs attention across a repo.
- **Broader Bitbucket coverage.** More of the Bitbucket API surface —
  deployments, project administration, permissions, and endpoints not yet
  wrapped in a dedicated command (the `bb api` passthrough covers them in the
  meantime).
- **More MCP tools.** Extending the server as the command surface grows.

Ideas and contributions in these directions are very welcome — open an issue to
start a conversation.

## Acknowledgements

This project stands on the shoulders of the original **[bb-cli](https://github.com/bb-cli/bb-cli)**,
a PHP tool created and maintained by **Semih Erdoğan**, with significant work by
**Dinçer Demircioğlu** and contributions from **Erşan Işık**, **Celal Akyüz**,
and others. Their design — the command structure, the config format, and the
overall UX — shaped this tool directly; the Rust version is a faithful port of
their work, undertaken to drop the runtime dependency and adopt Bitbucket's new
API-token authentication. Thank you for building the original and sharing it
openly. 🙏

## Contributing

Issues and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

The MIT License (MIT). Please see [the License file](LICENSE) for more
information.
