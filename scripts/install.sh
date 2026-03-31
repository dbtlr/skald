#!/usr/bin/env bash
set -euo pipefail

REPO="dbtlr/skald"
BINARY="sk"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

info() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
err()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# Detect OS
case "$(uname -s)" in
  Linux)  OS="unknown-linux-gnu" ;;
  Darwin) OS="apple-darwin" ;;
  *)      err "Unsupported OS: $(uname -s). Only Linux and macOS are supported." ;;
esac

# Detect architecture
case "$(uname -m)" in
  x86_64|amd64)  ARCH="x86_64" ;;
  aarch64|arm64)  ARCH="aarch64" ;;
  *)              err "Unsupported architecture: $(uname -m). Only x86_64 and aarch64 are supported." ;;
esac

TARGET="${ARCH}-${OS}"

# Determine version
if [ -n "${VERSION:-}" ]; then
  TAG="$VERSION"
else
  info "Fetching latest release..."
  TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | head -1 \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
  [ -n "$TAG" ] || err "Failed to determine latest release. Set VERSION env var to install a specific version."
fi

info "Installing ${BINARY} ${TAG} (${TARGET})"

ARCHIVE="${BINARY}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"

# Download and extract
TMPDIR_INSTALL="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_INSTALL"' EXIT

info "Downloading ${URL}..."
curl -fsSL "$URL" -o "${TMPDIR_INSTALL}/${ARCHIVE}" \
  || err "Download failed. Check that release ${TAG} exists for target ${TARGET}."

info "Extracting..."
tar xzf "${TMPDIR_INSTALL}/${ARCHIVE}" -C "$TMPDIR_INSTALL"

# Install
mkdir -p "$INSTALL_DIR"
mv "${TMPDIR_INSTALL}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"

info "Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

# Warn if not in PATH
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    printf '\n\033[1;33mwarning:\033[0m %s is not in your PATH.\n' "$INSTALL_DIR"
    printf 'Add it with:\n\n  export PATH="%s:$PATH"\n\n' "$INSTALL_DIR"
    ;;
esac
