#!/bin/sh
# tmosh installer — download the latest release binary for this host.
#
#   curl -fsSL https://raw.githubusercontent.com/totophe/tmosh/main/install.sh | sh
#
# Honors:
#   TMOSH_INSTALL_DIR   install location (default: ~/.local/bin)
#   TMOSH_VERSION       tag to install   (default: latest)

set -eu

REPO="totophe/tmosh"
INSTALL_DIR="${TMOSH_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${TMOSH_VERSION:-latest}"

err() { printf '\033[31merror:\033[0m %s\n' "$1" >&2; exit 1; }
info() { printf '\033[36m%s\033[0m\n' "$1" >&2; }

need() { command -v "$1" >/dev/null 2>&1 || err "missing required command: $1"; }
need uname
command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 \
  || err "need curl or wget"

# --- detect target triple --------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux)  os_part="unknown-linux-gnu" ;;
  Darwin) os_part="apple-darwin" ;;
  *) err "unsupported OS: $os" ;;
esac
case "$arch" in
  x86_64|amd64)  arch_part="x86_64" ;;
  aarch64|arm64) arch_part="aarch64" ;;
  *) err "unsupported architecture: $arch" ;;
esac
asset="tmosh-${arch_part}-${os_part}"

# --- resolve version -------------------------------------------------------
fetch() {
  # fetch URL -> stdout
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1"
  else
    wget -qO- "$1"
  fi
}
download() {
  # download URL DEST
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$2" "$1"
  else
    wget -qO "$2" "$1"
  fi
}

if [ "$VERSION" = "latest" ]; then
  info "Resolving latest release…"
  VERSION="$(fetch "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep -o '"tag_name"[ ]*:[ ]*"[^"]*"' \
    | head -n1 \
    | sed 's/.*"tag_name"[ ]*:[ ]*"\([^"]*\)".*/\1/')"
  [ -n "$VERSION" ] || err "could not resolve latest release tag"
fi

url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
info "Installing tmosh ${VERSION} (${asset})…"

# --- download & install ----------------------------------------------------
mkdir -p "$INSTALL_DIR"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT INT TERM
download "$url" "$tmp" || err "download failed: $url"
[ -s "$tmp" ] || err "downloaded file is empty"
chmod +x "$tmp"
mv "$tmp" "$INSTALL_DIR/tmosh"
trap - EXIT INT TERM

info "Installed to $INSTALL_DIR/tmosh"

# --- post-install hint -----------------------------------------------------
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) printf '\n\033[33mNote:\033[0m %s is not on your PATH. Add:\n  export PATH="%s:$PATH"\n' \
       "$INSTALL_DIR" "$INSTALL_DIR" >&2 ;;
esac

# Pick the right rc file for the hint.
rc="$HOME/.bashrc"
case "${SHELL:-}" in
  *zsh) rc="$HOME/.zshrc" ;;
esac

cat >&2 <<EOF

To launch tmosh on every interactive login, append its snippet to your shell rc:

  tmosh --init >> "$rc"

Then reload:  source "$rc"

Run 'tmosh' now to try it, or 'tmosh --help' for options.
EOF
