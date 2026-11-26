# Static musl build on distroless: no shell, no libc, runs as nonroot (65532).
# Build from the repo root: docker build -f crates/nf-llm-replay/Dockerfile -t nf-llm-replay .
FROM rust:1.100.0-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release --locked -p nf-llm-replay

FROM gcr.io/distroless/static-debian12:nonroot
LABEL org.opencontainers.image.source="https://github.com/pingedbrain/nf-agents" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.description="nf-llm-replay: record/replay proxy for LLM HTTP APIs (pre-alpha)"
COPY --from=build /src/target/release/nf-llm-replay /nf-llm-replay
USER 65532:65532
ENV NF_LLM_REPLAY_BIND=0.0.0.0 NF_LLM_REPLAY_PORT=8080
EXPOSE 8080
ENTRYPOINT ["/nf-llm-replay"]
