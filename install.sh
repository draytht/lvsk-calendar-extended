#!/bin/bash
set -e

echo "=== LifeManager Installer ==="

# Check for Rust
if ! command -v cargo &>/dev/null; then
  echo "Installing Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

echo "Building (this takes ~2 min first time)..."
cargo build --release

echo "Installing binary to ~/.local/bin/lm"
mkdir -p ~/.local/bin
cp target/release/lm ~/.local/bin/lm

echo "Setting up config..."
mkdir -p ~/.config/lifemanager
if [ ! -f ~/.config/lifemanager/config.toml ]; then
  cp config.example.toml ~/.config/lifemanager/config.toml
  echo "Config written to ~/.config/lifemanager/config.toml"
  echo "Edit it to add your Google client_id and client_secret."
fi

echo ""
echo "Done! Run:  lm"
