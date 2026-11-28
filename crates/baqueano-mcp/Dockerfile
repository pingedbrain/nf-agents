# Static musl build on distroless: no shell, no libc, runs as nonroot (65532). Speaks MCP over
# stdio, so run it with `docker run -i` and mount the lineage store read-only.
# Build from the repo root: docker build -f crates/baqueano-mcp/Dockerfile -t baqueano-mcp .
FROM rust:1.100.0-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release --locked -p baqueano-mcp

FROM gcr.io/distroless/static-debian12:nonroot
LABEL org.opencontainers.image.source="https://github.com/pingedbrain/baqueano" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.description="baqueano-mcp: read-only MCP server for Nextflow data lineage (pre-alpha)"
COPY --from=build /src/target/release/baqueano-mcp /baqueano-mcp
USER 65532:65532
ENTRYPOINT ["/baqueano-mcp"]
