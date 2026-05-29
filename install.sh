#!/usr/bin/env bash
# Continuum installer for macOS and Linux
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/NiteshYadavPataudi/continuum/main/install.sh | bash
#
# Environment variables:
#   CONTINUUM_VERSION      Version to install, or "latest" (default: latest)
#   CONTINUUM_INSTALL_DIR  Cargo install root (default: ~/.local/continuum)

set -euo pipefail

CONTINUUM_VERSION="${CONTINUUM_VERSION:-latest}"
INSTALL_DIR="${CONTINUUM_INSTALL_DIR:-${INSTALL_DIR:-$HOME/.local/continuum}}"
PROFILE_FILE="${PROFILE_FILE:-}"

info() { printf '\033[36m%s\033[0m\n' "$*"; }
warn() { printf '\033[33m%s\033[0m\n' "$*"; }
ok() { printf '\033[32m%s\033[0m\n' "$*"; }
err() { printf '\033[31m%s\033[0m\n' "$*" >&2; }

path_contains() {
    case ":$PATH:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

append_path_to_profile() {
    local dir="$1"
    local profile="$2"
    local marker_begin="# >>> Continuum >>>"
    local marker_end="# <<< Continuum <<<"
    local line="export PATH=\"$dir:\$PATH\""

    mkdir -p "$(dirname "$profile")"
    touch "$profile"

    if grep -Fq "$marker_begin" "$profile"; then
        return 0
    fi

    {
        printf '\n%s\n' "$marker_begin"
        printf '%s\n' "$line"
        printf '%s\n' "$marker_end"
    } >> "$profile"
}

profile_for_shell() {
    if [ -n "$PROFILE_FILE" ]; then
        printf '%s' "$PROFILE_FILE"
        return 0
    fi

    case "${SHELL:-}" in
        */zsh) printf '%s' "$HOME/.zshrc" ;;
        */bash) printf '%s' "$HOME/.bashrc" ;;
        *) printf '%s' "$HOME/.profile" ;;
    esac
}

info "Installing Continuum..."
printf '\n'

if ! command -v cargo >/dev/null 2>&1; then
    err "Rust/Cargo was not found."
    printf 'Install Rust from: https://rustup.rs\n'
    exit 1
fi

mkdir -p "$INSTALL_DIR"

info "Install root: $INSTALL_DIR"
info "Version: $CONTINUUM_VERSION"
printf '\n'

if [ "$CONTINUUM_VERSION" = "latest" ]; then
    cargo install --locked --root "$INSTALL_DIR" continuum-cli
else
    cargo install --locked --root "$INSTALL_DIR" continuum-cli --version "$CONTINUUM_VERSION"
fi

if ! path_contains "$INSTALL_DIR"; then
    printf '\n'
    warn "Adding $INSTALL_DIR to PATH for this shell..."
    export PATH="$INSTALL_DIR:$PATH"

    profile="$(profile_for_shell)"
    append_path_to_profile "$INSTALL_DIR" "$profile"
    ok "Updated PATH in $profile"
    printf 'Open a new terminal, or run: source %s\n' "$profile"
fi

printf '\n'
ok "Installation complete!"
printf '\n'
printf 'Verify installation:\n'
printf '  continuum --version\n'
printf '  continuum doctor\n'
printf '\n'
