# syntax=docker/dockerfile:1.6
#
# Rocky demo / dev image
#
#   Stage 1 (builder)  — compile rocky in release mode
#   Stage 2 (whisper)  — compile whisper.cpp + download base.en model
#   Stage 3 (runtime)  — slim debian with rocky, whisper-cli, toy project, scripts
#
# Build:   docker build -f docker/Dockerfile -t rocky:demo .
# Compose: docker compose -f docker/docker-compose.yml --profile demo up
#
# ────────────────────────────────────────────────────────────────────────────
# Stage 1: Rust builder
# ────────────────────────────────────────────────────────────────────────────
FROM rust:1-bookworm AS rocky-builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# ────────────────────────────────────────────────────────────────────────────
# Stage 2: whisper.cpp builder
# ────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS whisper-builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        git \
        build-essential \
        cmake \
        ca-certificates \
        curl \
        && rm -rf /var/lib/apt/lists/*

WORKDIR /opt
RUN git clone --depth=1 https://github.com/ggerganov/whisper.cpp.git
WORKDIR /opt/whisper.cpp
RUN cmake -B build -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF && \
    cmake --build build --config Release -j"$(nproc)" --target whisper-cli && \
    bash ./models/download-ggml-model.sh base.en

# ────────────────────────────────────────────────────────────────────────────
# Stage 3: runtime
# ────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

ARG NODE_MAJOR=20

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        gnupg \
        git \
        sqlite3 \
        less \
        jq \
        procps \
        tini \
        bash \
        libgomp1 \
        && curl -fsSL https://deb.nodesource.com/setup_${NODE_MAJOR}.x | bash - \
        && apt-get install -y --no-install-recommends nodejs \
        && rm -rf /var/lib/apt/lists/*

# whisper.cpp binary + model
COPY --from=whisper-builder /opt/whisper.cpp/build/bin/whisper-cli /usr/local/bin/whisper-cli
RUN mkdir -p /opt/whisper-models
COPY --from=whisper-builder /opt/whisper.cpp/models/ggml-base.en.bin /opt/whisper-models/ggml-base.en.bin

# rocky binary
COPY --from=rocky-builder /build/target/release/rocky /usr/local/bin/rocky

# Toy project + entrypoints + config template
COPY docker/toy-project /opt/toy-project
COPY docker/entrypoint-demo.sh /usr/local/bin/entrypoint-demo
COPY docker/entrypoint-dev.sh /usr/local/bin/entrypoint-dev
COPY docker/config-template.toml /opt/config-template.toml
RUN chmod +x /usr/local/bin/entrypoint-demo /usr/local/bin/entrypoint-dev

# Non-root user with passwordless sudo-less setup. UID 1000 keeps mounted
# volumes friendly on Linux hosts where the developer is also UID 1000.
RUN useradd -m -u 1000 -s /bin/bash rocky && \
    mkdir -p /home/rocky/.config/rocky /home/rocky/.rocky /workspace && \
    chown -R rocky:rocky /home/rocky /workspace /opt/toy-project

USER rocky
WORKDIR /workspace
ENV HOME=/home/rocky \
    PATH=/usr/local/bin:/usr/bin:/bin:/home/rocky/.local/bin \
    ROCKY_OLLAMA_HOST=host.docker.internal:11434

# Seed git identity so commits inside the demo work without prompting.
RUN git config --global user.email "demo@rocky.local" && \
    git config --global user.name "Rocky Demo" && \
    git config --global init.defaultBranch main

EXPOSE 7777

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/entrypoint-dev"]
