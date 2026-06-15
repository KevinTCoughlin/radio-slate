# radio-slate

Fresh local Rust radio player for Linux desktop development.

This project uses a small clean-architecture Rust layout with a CLI-first workflow and a local tray-menu path for quick playback testing on Fedora/Linux.

- domain: station models and playback rules
- application: playback orchestration and service behavior
- infrastructure: repository and playback adapters
- ui: CLI and tray entry points

## Development workflow

```sh
just fmt
just check
just clippy
just test
just package
just verify-release
just run
just list
```

## Linux build dependencies

Fedora development packages:

```sh
sudo dnf install gtk3-devel libappindicator-gtk3-devel pkgconf-pkg-config
```

Ubuntu/GitHub Actions development packages:

```sh
sudo apt-get install libgtk-3-dev libappindicator3-dev pkg-config
```

## Tray mode

```sh
cargo run -- --tray
# or after install
~/.cargo/bin/radio-slate --tray
```

The tray path uses the local AppIndicator/GTK menu for toggling playback and quitting the app.
It attempts playback with `mpv` first and falls back to `ffplay` if `mpv` is unavailable.

### Runtime dependencies (Linux)

- GTK 3 and AppIndicator development/runtime libraries (`gtk+-3.0`, `libappindicator3`)
- At least one supported player on your `PATH`:
  - `mpv` (preferred)
  - `ffplay` (fallback)

## Local install

```sh
cargo install --path . --locked
~/.cargo/bin/radio-slate --play
~/.cargo/bin/radio-slate --list --format json
```

## Release verification

Validate the same path used by CI and the release workflow:

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo package --locked
cargo install --path . --locked --root /tmp/radio-slate-install
/tmp/radio-slate-install/bin/radio-slate --list --format json
```

The release workflow publishes two Linux artifacts on version tags:

- `radio-slate-linux-x86_64.tar.gz` containing the release binary
- `radio-slate-<version>.crate` containing the packaged crate source

## Default station

By default, the app starts from:

```text
http://live-mp3-128.kexp.org/kexp128.mp3
```

You can customize local settings in:

- `${XDG_CONFIG_HOME:-$HOME/.config}/radio-slate/config.json`
- `${XDG_STATE_HOME:-$HOME/.local/state}/radio-slate/state.json`

`config.json` supports:

- `volume_percent`
- `default_station_url`
- `tray_autoplay`
- `tray_icon`

`state.json` stores `last_station_url` so startup can restore your previous station.

## Editor support

- Zed: `.zed/settings.json` includes rust-analyzer and format-on-save defaults.
- Rust toolchain: the project uses Rust 2024 and pins Rust 1.96 via `rust-version`.
