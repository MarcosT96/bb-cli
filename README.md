# bb — Bitbucket CLI

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
The command surface mirrors the original tool — `auth`, `branch`, `browse`,
`env`, `pipeline`, `pr`, `pr-details`, and `upgrade`.

## Roadmap

The rewrite to a dependency-free Rust binary is the foundation for where this
project is headed. The direction — not a set of dated promises, but the way we
want to grow it:

- **An MCP server.** Expose Bitbucket to AI assistants through the
  [Model Context Protocol](https://modelcontextprotocol.io), so tools like
  Claude can read PRs, inspect pipelines, and act on a repo through a safe,
  typed interface — reusing the same client this CLI is built on.
- **AI-assisted workflows.** Summarizing pull requests and review threads,
  drafting PR descriptions, triaging pipeline failures, and surfacing what needs
  attention across a repo.
- **Broader Bitbucket coverage.** More of the Bitbucket API surface —
  deployments, repository and project administration, webhooks, permissions,
  and other endpoints not yet wrapped.

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
