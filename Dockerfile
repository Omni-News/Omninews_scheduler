# syntax=docker/dockerfile:1

# ============ Builder ============
FROM rust:1.89-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
      g++ pkg-config libssl-dev libmariadb-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 의존성만 먼저 빌드해 레이어 캐시를 남긴다.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
 && cargo build --release \
 && rm -rf src target/release/OmniNews target/release/deps/OmniNews*

# query! 매크로를 DB 없이 검증하기 위해 저장소의 쿼리 캐시를 쓴다.
ENV SQLX_OFFLINE=1
COPY .sqlx ./.sqlx
COPY src ./src
COPY Rocket.toml ./
RUN cargo build --release

# ============ Runtime ============
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates libssl3 libmariadb3 \
 && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 omninews
WORKDIR /app

COPY --from=builder /app/target/release/OmniNews /app/omninews_scheduler
COPY Rocket.toml ./Rocket.toml
RUN mkdir -p /app/logs && chown -R omninews:omninews /app

USER omninews
EXPOSE 1029
CMD ["/app/omninews_scheduler"]
