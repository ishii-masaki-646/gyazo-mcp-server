FROM rust:1-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release && \
    strip target/release/gyazo-mcp-server

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/gyazo-mcp-server /usr/local/bin/

EXPOSE 18449

ENTRYPOINT ["gyazo-mcp-server"]
