# Bitbucket Rest API CLI

Use Bitbucket from command line. With this app you can see pull request, pipelines, branches etc. from your terminal.

![Bitbucket CLI](ss.gif)

`bb` is a single static binary — no runtime to install.

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

[View the documentation](https://bb-cli.github.io) for usage information.

## Development

This tool developed with help of [Github Copilot](https://copilot.github.com) :octocat: - 2021

## License

The MIT License (MIT). Please see [License File](LICENSE) for more information.
