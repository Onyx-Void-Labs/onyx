# Build Stage
FROM rust:slim AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# 1. Cache dependencies — only re-runs if Cargo.toml/Cargo.lock change
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# 2. Build real source — only this layer rebuilds on code changes (~10s)
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Runtime Stage (minimal image)
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/onyx-relay /usr/local/bin/onyx-relay
ENV PORT=3000
EXPOSE 3000
CMD ["onyx-relay"]
