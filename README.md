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

The tray path uses the local AppIndicator/GTK menu as a thin adapter over the same playback service used by the CLI.

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
