# syntax=docker/dockerfile:1.7

# ---- Build -----------------------------------------------------------------
ARG RUST_VERSION=1.90
FROM rust:${RUST_VERSION}-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates ./crates
COPY web ./web
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p libreplume-server \
 && cp target/release/libreplume /usr/local/bin/libreplume
RUN mkdir -p /data

# ---- Runtime: distroless (glibc, no shell, non-root) -----------------------
FROM gcr.io/distroless/cc-debian12:nonroot
LABEL org.opencontainers.image.title="LibrePlume" \
      org.opencontainers.image.description="Grammar, spelling, conjugation, typography and style checker with web UI and MCP server" \
      org.opencontainers.image.source="https://github.com/pmorcrette/LibrePlume" \
      org.opencontainers.image.licenses="GPL-3.0-or-later"
COPY --from=build /usr/local/bin/libreplume /usr/local/bin/libreplume
COPY --from=build --chown=nonroot:nonroot /data /data
ENV LIBREPLUME_DATA_DIR=/data \
    LIBREPLUME_HOST=0.0.0.0 \
    LIBREPLUME_PORT=8080
EXPOSE 8080
VOLUME ["/data"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s CMD ["libreplume", "healthcheck"]
ENTRYPOINT ["libreplume"]
CMD ["serve"]
