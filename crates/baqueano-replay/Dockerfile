# Static musl build on distroless: no shell, no libc, runs as nonroot (65532).
# Build from the repo root: docker build -f crates/baqueano-replay/Dockerfile -t baqueano-replay .
FROM rust:1.100.0-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release --locked -p baqueano-replay

FROM gcr.io/distroless/static-debian12:nonroot
LABEL org.opencontainers.image.source="https://github.com/pingedbrain/baqueano" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.description="baqueano-replay: record/replay proxy for LLM HTTP APIs (pre-alpha)"
COPY --from=build /src/target/release/baqueano-replay /baqueano-replay
USER 65532:65532
ENV BAQUEANO_REPLAY_BIND=0.0.0.0 BAQUEANO_REPLAY_PORT=8080
EXPOSE 8080
ENTRYPOINT ["/baqueano-replay"]
