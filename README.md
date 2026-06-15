# radio-slate

Fresh local Rust radio player for Linux desktop development.

This project uses a small clean-architecture Rust layout with a CLI-first workflow and a local tray-menu path for quick playback testing on Fedora/Linux.

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

## Default station

The current default test stream is pinned to KEXP:

```text
http://live-mp3-128.kexp.org/kexp128.mp3
```

## Editor support

- Zed: `.zed/settings.json` includes rust-analyzer and format-on-save defaults.
- Rust toolchain: the project uses Rust 2024 and pins Rust 1.96 via `rust-version`.
