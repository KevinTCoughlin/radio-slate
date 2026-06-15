FROM rust:1.96-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libgtk-3-dev \
        libappindicator3-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        ffmpeg \
        libappindicator3-1 \
        libgtk-3-0 \
        mpv \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/radio-slate /usr/local/bin/radio-slate

ENTRYPOINT ["/usr/local/bin/radio-slate"]
