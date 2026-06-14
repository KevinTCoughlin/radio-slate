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

run:
    cargo run -- --play

list:
    cargo run -- --list --format json

install:
    cargo install --path . --locked

clean:
    cargo clean
