# syntax=docker/dockerfile:1

# ============ Builder ============
FROM rust:1.89-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
      g++ pkg-config libssl-dev libmariadb-dev \
      python3 python3-venv libgfortran5 ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# PyTorch는 linux/arm64용 libtorch C++ 배포본을 제공하지 않는다.
# PyPI wheel 안에 완전한 libtorch가 들어 있으므로 그것을 쓴다.
# torch 2.4.0은 cp312까지만 wheel이 있어 베이스를 bookworm(Python 3.11)으로 고정한다.
RUN python3 -m venv /opt/venv \
 && /opt/venv/bin/pip install --no-cache-dir torch==2.4.0

RUN TORCH_DIR="$(/opt/venv/bin/python -c 'import torch,os;print(os.path.dirname(torch.__file__))')" \
 && ln -s "$TORCH_DIR" /opt/libtorch \
 && ln -s "$(dirname "$TORCH_DIR")/torch.libs" /opt/torch-libs

ENV LIBTORCH=/opt/libtorch \
    LD_LIBRARY_PATH=/opt/libtorch/lib:/opt/torch-libs \
    LIBTORCH_BYPASS_VERSION_CHECK=1 \
    LIBTORCH_CXX11_ABI=0 \
    RUSTFLAGS="-L/opt/torch-libs"

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
      ca-certificates libssl3 libmariadb3 libgomp1 libgfortran5 \
 && rm -rf /var/lib/apt/lists/*

# 공유 라이브러리만 옮긴다. python 패키지 본체는 런타임에 필요 없다.
COPY --from=builder /opt/libtorch/lib /opt/libtorch/lib
COPY --from=builder /opt/torch-libs /opt/torch-libs
ENV LD_LIBRARY_PATH=/opt/libtorch/lib:/opt/torch-libs

RUN useradd -m -u 1000 omninews
WORKDIR /app

COPY --from=builder /app/target/release/OmniNews /app/omninews_scheduler
COPY Rocket.toml ./Rocket.toml
RUN mkdir -p /app/logs && chown -R omninews:omninews /app

USER omninews
EXPOSE 1029
CMD ["/app/omninews_scheduler"]
