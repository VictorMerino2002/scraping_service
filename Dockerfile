# syntax=docker/dockerfile:1

ARG CHROMIUM_VERSION=v149.0.0
ARG CARGO_LAMBDA_VERSION=v1.9.1
ARG ZIG_VERSION=0.14.1
ARG LAMBDA_ARCH=arm64
ARG BIN_NAME=http_handler

# ---- Fetch and unpack the Lambda-compatible headless Chromium build ----
# https://github.com/Sparticuz/chromium ships chromium + its shared libs
# brotli-compressed; we decompress once here at build time instead of at
# every cold start (which is what their JS wrapper does).
FROM debian:bookworm-slim AS chromium
ARG CHROMIUM_VERSION
ARG LAMBDA_ARCH
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl brotli ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /chromium
RUN set -eux; \
    arch_suffix="$([ "$LAMBDA_ARCH" = "arm64" ] && echo arm64 || echo x64)"; \
    curl -sSfL -o pack.tar \
        "https://github.com/Sparticuz/chromium/releases/download/${CHROMIUM_VERSION}/chromium-${CHROMIUM_VERSION}-pack.${arch_suffix}.tar"; \
    tar -xf pack.tar; \
    brotli -d chromium.br -o chromium; \
    brotli -d al2023.tar.br -o al2023.tar; \
    brotli -d fonts.tar.br -o fonts.tar; \
    brotli -d swiftshader.tar.br -o swiftshader.tar; \
    mkdir -p /opt/chromium/fonts /opt/chromium/swiftshader; \
    mv chromium /opt/chromium/chromium; \
    chmod +x /opt/chromium/chromium; \
    tar -xf al2023.tar -C /opt/chromium; \
    tar -xf fonts.tar -C /opt/chromium/fonts; \
    tar -xf swiftshader.tar -C /opt/chromium/swiftshader

# ---- Build the Rust Lambda binary ----
FROM rust:1-bookworm AS builder
ARG CARGO_LAMBDA_VERSION
ARG ZIG_VERSION
ARG BIN_NAME
# cargo-lambda cross-compiles to arm64 via Zig, regardless of the builder's
# own (amd64) host architecture.
RUN set -eux; \
    curl -sSfL -o /tmp/zig.tar.xz \
        "https://ziglang.org/download/${ZIG_VERSION}/zig-x86_64-linux-${ZIG_VERSION}.tar.xz"; \
    mkdir -p /opt/zig; \
    tar -xJf /tmp/zig.tar.xz -C /opt/zig --strip-components=1; \
    ln -s /opt/zig/zig /usr/local/bin/zig; \
    rm /tmp/zig.tar.xz; \
    curl -sSfL -o /tmp/cargo-lambda.tar.gz \
        "https://github.com/cargo-lambda/cargo-lambda/releases/download/${CARGO_LAMBDA_VERSION}/cargo-lambda-${CARGO_LAMBDA_VERSION}.x86_64-unknown-linux-musl.tar.gz"; \
    tar -xzf /tmp/cargo-lambda.tar.gz -C /usr/local/bin cargo-lambda; \
    rm /tmp/cargo-lambda.tar.gz
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo lambda build --release --arm64 --bin ${BIN_NAME}

# ---- Final Lambda image ----
FROM public.ecr.aws/lambda/provided:al2023-arm64
ARG BIN_NAME
COPY --from=chromium /opt/chromium /opt/chromium
COPY --from=builder /build/target/lambda/${BIN_NAME}/bootstrap /var/runtime/bootstrap
ENV CHROME_EXECUTABLE_PATH=/opt/chromium/chromium
ENV LD_LIBRARY_PATH=/opt/chromium/lib:${LD_LIBRARY_PATH}
CMD ["bootstrap"]
