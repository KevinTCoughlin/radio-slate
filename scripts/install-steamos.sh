#!/usr/bin/env bash
set -euo pipefail

AUTOSTART=0

while (($# > 0)); do
  case "$1" in
    --enable-autostart)
      AUTOSTART=1
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Usage: install-steamos.sh [--enable-autostart]

Installs radio-slate for Steam Deck / SteamOS Desktop Mode using cargo,
then installs a launcher in ~/.local/share/applications.

Options:
  --enable-autostart   also install an XDG autostart entry in ~/.config/autostart
EOF
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required to install radio-slate on SteamOS." >&2
  echo "Install Rust tooling first, then rerun this script." >&2
  exit 1
fi

if ! command -v mpv >/dev/null 2>&1 && ! command -v ffplay >/dev/null 2>&1; then
  echo "radio-slate needs either mpv or ffplay available in PATH." >&2
  echo "Install one of them in Desktop Mode, then rerun this script." >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
RADIO_SLATE_BIN="$CARGO_BIN_DIR/radio-slate"
APPLICATIONS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
AUTOSTART_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"

cargo install --path "$REPO_ROOT" --locked --force

if [[ ! -x "$RADIO_SLATE_BIN" ]]; then
  echo "Expected installed binary at $RADIO_SLATE_BIN but it was not found." >&2
  exit 1
fi

mkdir -p "$APPLICATIONS_DIR"
sed "s|@RADIO_SLATE_BIN@|$RADIO_SLATE_BIN|g" \
  "$REPO_ROOT/assets/radio-slate.desktop" \
  > "$APPLICATIONS_DIR/radio-slate.desktop"

if ((AUTOSTART)); then
  mkdir -p "$AUTOSTART_DIR"
  sed "s|@RADIO_SLATE_BIN@|$RADIO_SLATE_BIN|g" \
    "$REPO_ROOT/assets/radio-slate-autostart.desktop" \
    > "$AUTOSTART_DIR/radio-slate.desktop"
fi

cat <<EOF
Installation complete.

Desktop launcher:
  $APPLICATIONS_DIR/radio-slate.desktop
EOF

if ((AUTOSTART)); then
  cat <<EOF
Autostart enabled:
  $AUTOSTART_DIR/radio-slate.desktop
EOF
else
  cat <<'EOF'
Autostart not enabled.
Re-run with --enable-autostart to start the tray automatically on login.
EOF
fi
