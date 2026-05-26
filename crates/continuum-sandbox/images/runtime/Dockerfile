FROM rust:1.78-slim-bookworm AS base

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev git clang lld \
    nodejs npm python3 python3-pip python3-venv \
    && rm -rf /var/lib/apt/lists/*

FROM base AS toolchain

RUN rustup component add rustfmt clippy

RUN npm install -g typescript eslint prettier 2>&1 || true

RUN python3 -m venv /opt/venv && \
    /opt/venv/bin/pip install black ruff mypy pytest pytest-cov 2>&1 || true

ENV PATH="/opt/venv/bin:${PATH}"

FROM toolchain AS sandbox

RUN mkdir -p /workspace /cache/registry /cache/compiledb

WORKDIR /workspace

COPY sandbox-entrypoint.sh /usr/local/bin/sandbox-entrypoint.sh
RUN chmod +x /usr/local/bin/sandbox-entrypoint.sh

ENV CARGO_HOME=/cache/registry
ENV RUSTUP_HOME=/cache/rustup

ENTRYPOINT ["sandbox-entrypoint.sh"]
