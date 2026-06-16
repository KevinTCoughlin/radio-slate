#!/usr/bin/env bash
set -euo pipefail

if ! command -v sudo >/dev/null 2>&1; then
  echo "sudo is required to install Ubuntu dependencies." >&2
  exit 1
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "This installer targets Ubuntu/Debian-style systems with apt-get." >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ ! -f "$REPO_ROOT/Cargo.toml" ]]; then
  echo "Error: This script must be run from a cloned radio-slate repository." >&2
  echo "Example:" >&2
  echo "  git clone https://github.com/KevinTCoughlin/radio-slate.git" >&2
  echo "  cd radio-slate" >&2
  echo "  bash scripts/install-ubuntu.sh" >&2
  exit 1
fi

sudo apt-get update
sudo apt-get install -y \
  build-essential \
  ca-certificates \
  clang \
  curl \
  ffmpeg \
  libappindicator3-dev \
  libgtk-3-dev \
  mpv \
  pkg-config

if ! command -v cargo >/dev/null 2>&1; then
  curl https://sh.rustup.rs -sSf | sh -s -- -y
  # shellcheck disable=SC1090
  source "${CARGO_HOME:-$HOME/.cargo}/env"
fi

cargo install --path "$REPO_ROOT" --locked --force

cat <<'EOF'
Installation complete.

Try:
  ~/.cargo/bin/radio-slate --list --format json
  ~/.cargo/bin/radio-slate --play
EOF
