default:
    just --list

fmt:
    cargo fmt --all

check:
    cargo check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test

package:
    cargo package --locked

verify-release:
    cargo install --path . --locked --root /tmp/radio-slate-install
    /tmp/radio-slate-install/bin/radio-slate --list --format json

run:
    cargo run -- --play

list:
    cargo run -- --list --format json

install:
    cargo install --path . --locked

clean:
    cargo clean
