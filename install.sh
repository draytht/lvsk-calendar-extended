#!/bin/bash
set -e

echo "=== LifeManager Installer ==="

# ── Detect OS ─────────────────────────────────────────────────────────────────
OS="$(uname -s)"
case "$OS" in
  Linux)  PLATFORM="linux" ;;
  Darwin) PLATFORM="macos" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

# ── macOS: require Xcode Command Line Tools (needed to compile native deps) ───
if [ "$PLATFORM" = "macos" ]; then
  if ! xcode-select -p &>/dev/null; then
    echo "Xcode Command Line Tools are required. Installing..."
    xcode-select --install
    echo "Re-run this script after the installation completes."
    exit 1
  fi
fi

# ── Check for Rust ─────────────────────────────────────────────────────────────
if ! command -v cargo &>/dev/null; then
  echo "Installing Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

echo "Building (this takes ~2 min first time)..."
cargo build --release

# ── Platform-specific paths ────────────────────────────────────────────────────
BIN_DIR="$HOME/.local/bin"

if [ "$PLATFORM" = "macos" ]; then
  # Must match what dirs::config_dir() returns on macOS
  CONFIG_DIR="$HOME/Library/Application Support/lifemanager"
else
  # Must match what dirs::config_dir() returns on Linux
  CONFIG_DIR="$HOME/.config/lifemanager"
fi

# ── Install binary ─────────────────────────────────────────────────────────────
echo "Installing binary to $BIN_DIR/lm"
mkdir -p "$BIN_DIR"
cp target/release/lm "$BIN_DIR/lm"

# Ensure BIN_DIR is in PATH (common issue on macOS)
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
  echo ""
  echo "NOTE: $BIN_DIR is not in your PATH."

  # Pick the right shell profile
  if [ "$PLATFORM" = "macos" ]; then
    # macOS default shell is zsh since Catalina; use .zprofile for login shells
    SHELL_PROFILE="$HOME/.zprofile"
    [ "$(basename "$SHELL")" = "bash" ] && SHELL_PROFILE="$HOME/.bash_profile"
  elif [ "$(basename "$SHELL")" = "zsh" ]; then
    SHELL_PROFILE="$HOME/.zshrc"
  else
    SHELL_PROFILE="$HOME/.bashrc"
  fi

  echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_PROFILE"
  echo "Added to $SHELL_PROFILE — restart your terminal or run: source $SHELL_PROFILE"
fi

# ── Setup config ───────────────────────────────────────────────────────────────
echo "Setting up config..."
mkdir -p "$CONFIG_DIR"
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
  cp config.example.toml "$CONFIG_DIR/config.toml"
  echo "Config written to: $CONFIG_DIR/config.toml"
  echo "Edit it to add your Google client_id and client_secret."
fi

echo ""
echo "Done! Run:  lm"
