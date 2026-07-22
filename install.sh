#!/bin/sh
# bb-cli installer — downloads the latest release binary for your platform.
# Usage: curl -fsSL https://raw.githubusercontent.com/MarcosT96/bb-cli/main/install.sh | sh
set -eu

REPO="MarcosT96/bb-cli"
BIN="bb"

# --- Detect OS ---
os="$(uname -s)"
case "$os" in
  Darwin) os_part="apple-darwin" ;;
  Linux)  os_part="unknown-linux-gnu" ;;
  *)
    echo "Error: unsupported OS '$os'. Prebuilt binaries exist only for macOS and Linux." >&2
    echo "Install from source instead: cargo install --git https://github.com/$REPO" >&2
    exit 1
    ;;
esac

# --- Detect arch ---
arch="$(uname -m)"
case "$arch" in
  arm64|aarch64) arch_part="aarch64" ;;
  x86_64|amd64)  arch_part="x86_64" ;;
  *)
    echo "Error: unsupported architecture '$arch'. Prebuilt binaries exist only for x86_64 and aarch64/arm64." >&2
    echo "Install from source instead: cargo install --git https://github.com/$REPO" >&2
    exit 1
    ;;
esac

target="${arch_part}-${os_part}"
asset="${BIN}-${target}"
url="https://github.com/${REPO}/releases/latest/download/${asset}"

echo "Detected platform: ${target}"
echo "Downloading ${asset} ..."

tmp="$(mktemp)"
if ! curl -fsSL "$url" -o "$tmp"; then
  echo "Error: download failed from $url" >&2
  echo "No prebuilt binary for '${target}' (the built targets are:" >&2
  echo "  aarch64-apple-darwin, x86_64-apple-darwin," >&2
  echo "  x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu)." >&2
  rm -f "$tmp"
  exit 1
fi

chmod +x "$tmp"

# --- Install location ---
# Prefer /usr/local/bin; fall back to ~/.local/bin if it isn't writable.
dest_dir="/usr/local/bin"
if [ ! -w "$dest_dir" ] && [ "$(id -u)" -ne 0 ]; then
  if command -v sudo >/dev/null 2>&1; then
    echo "Installing to $dest_dir (requires sudo) ..."
    sudo mv "$tmp" "${dest_dir}/${BIN}"
  else
    dest_dir="${HOME}/.local/bin"
    mkdir -p "$dest_dir"
    mv "$tmp" "${dest_dir}/${BIN}"
    echo "Installed to ${dest_dir}/${BIN}"
    echo "Note: make sure '${dest_dir}' is on your PATH:"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
    echo "bb installed. Run 'bb --help' to get started."
    exit 0
  fi
else
  mv "$tmp" "${dest_dir}/${BIN}"
fi

echo "Installed ${BIN} to ${dest_dir}/${BIN}"
echo "Run 'bb --help' to get started."
