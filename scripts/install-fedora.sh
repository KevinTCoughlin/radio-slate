#!/usr/bin/env bash
set -euo pipefail

if ! command -v sudo >/dev/null 2>&1; then
  echo "sudo is required to install Fedora dependencies." >&2
  exit 1
fi

sudo dnf update -y
sudo dnf install -y \
  cargo \
  clang \
  ffmpeg \
  gcc \
  gtk3-devel \
  libappindicator-gtk3-devel \
  mpv \
  pkgconf-pkg-config

cargo install --path . --locked

cat <<'EOF'
Installation complete.

Try:
  ~/.cargo/bin/radio-slate --list --format json
  ~/.cargo/bin/radio-slate --play
EOF
