# radio-slate

Fresh local Rust radio player for Linux desktop development.

This project uses a small clean-architecture Rust layout with a CLI-first workflow and a local tray-menu path for quick playback testing on Fedora, Ubuntu, and WSL2.

- domain: station models and playback rules
- application: playback orchestration and service behavior
- infrastructure: repository, playback adapters, MPRIS D-Bus service, desktop notifications
- ui: CLI and tray entry points

## Development workflow

```sh
just fmt
just check
just test
just run
just list
```

## Tray mode

```sh
cargo run -- --tray
# or after install
~/.cargo/bin/radio-slate --tray
```

The tray path uses the local AppIndicator/GTK menu for toggling playback and quitting the app.
It also supports lightweight local desktop notifications (`notify-send` when available),
shows station metadata with safe fallback labels, and accepts keyboard/media shortcuts:
- play/pause: `Space`, `P`, `K`, `XF86AudioPlay`, `XF86AudioPause`
- next station: `N`, `XF86AudioNext`

### MPRIS / media keys

When tray mode starts, radio-slate registers itself on the session D-Bus as
`org.mpris.MediaPlayer2.radio-slate`.  This means:

- **GNOME Shell** displays a "Now Playing" widget in the top bar and on the lock
  screen, showing the station name and play/stop controls.
- **Hardware media keys** (⏯ Play/Pause, ⏹ Stop) are automatically routed
  through the MPRIS interface — no extra key-binding configuration needed.
- **`playerctl`** and other MPRIS clients can control playback:
  ```sh
  playerctl --player radio-slate play-pause
  playerctl --player radio-slate stop
  playerctl --player radio-slate status
  ```

The MPRIS service starts gracefully and falls back to tray-only mode when no
session D-Bus is available (SSH sessions, headless CI, etc.).

### Steam Deck / SteamOS Desktop Mode

Steam Deck support currently targets **Desktop Mode**.

- The existing MPRIS service is the primary integration surface on SteamOS, so
  KDE media controls, `playerctl`, and other MPRIS-aware clients can control
  playback once tray mode is running.
- Game Mode is not yet a first-class UI target because it does not reliably
  expose the GTK/AppIndicator tray surface used by the current app.
- The CLI snapshot/output path can serve as a future machine-readable interface
  for a Decky Loader plugin or other Steam Deck-specific frontend.

Install for Desktop Mode with:

```sh
bash scripts/install-steamos.sh
```

This expects Cargo plus either `mpv` or `ffplay` to already be available in
Desktop Mode. It installs the binary with Cargo and writes a desktop launcher
to:

- `~/.local/share/applications/radio-slate.desktop`

To also start the tray automatically on login:

```sh
bash scripts/install-steamos.sh --enable-autostart
```

That additionally writes:

- `~/.config/autostart/radio-slate.desktop`

### Desktop notifications

A brief `libnotify`-style desktop notification is sent whenever playback starts
or stops.  Notifications are silently suppressed when no notification daemon is
running.

### PipeWire / PulseAudio

Audio output is handled by `mpv` (preferred) with an automatic fallback to
`ffplay`.  Both players use the system audio stack transparently, so
PipeWire and PulseAudio environments are supported out of the box.

## Local install

```sh
cargo install --path . --locked
~/.cargo/bin/radio-slate --play
~/.cargo/bin/radio-slate --list --format json
```

## Sponsor

If you want to support ongoing Linux desktop tooling and open-source maintenance, sponsor the project via GitHub Sponsors:

- https://github.com/sponsors/kevintcoughlin

## Fedora install helper

For a one-command install on Fedora Linux, use the helper script:

```sh
curl -fsSL https://raw.githubusercontent.com/KevinTCoughlin/radio-slate/main/scripts/install-fedora.sh | bash
```

The script installs the Fedora build/runtime prerequisites (`cargo`, `mpv`, `ffmpeg`, GTK/AppIndicator development headers), then installs the binary into `~/.cargo/bin`.

## Ubuntu / WSL install helper

For Ubuntu (including WSL2 Ubuntu), first clone the repository, then run:

```sh
git clone https://github.com/KevinTCoughlin/radio-slate.git
cd radio-slate
bash scripts/install-ubuntu.sh
```

The helper installs apt prerequisites (`build-essential`, `clang`, `ffmpeg`,
GTK/AppIndicator development headers, `mpv`) and bootstraps Rust via `rustup`
if `cargo` is missing.

### WSL notes

- **CLI playback** works in standard WSL2 shells.
- **Tray mode** (`--tray`) needs a Linux GUI session. On Windows 11, that
  means WSLg enabled; otherwise use CLI mode.
- **MPRIS/media keys** depend on a desktop session D-Bus and may be unavailable
  in headless WSL terminals.

## GitHub Pages site

The repository includes a simple marketing site in `docs/` and an automated Pages deployment workflow in `.github/workflows/pages.yml`.

## Containerized build

```sh
podman build -t radio-slate .
podman run --rm -it localhost/radio-slate --list --format json
```

The container path is intended for reproducible builds and CLI workflows. Tray/desktop integration still uses the host GTK/AppIndicator session.

## Default station

The current default test stream is pinned to KEXP:

```text
http://live-mp3-128.kexp.org/kexp128.mp3
```

## Editor support

- Zed: `.zed/settings.json` includes rust-analyzer and format-on-save defaults.
- Rust toolchain: the project uses Rust 2024 and pins Rust 1.96 via `rust-version`.
