//! Deterministic fake LLM provider on `127.0.0.1:<port>` (no key needed), used to record the
//! offline example cassette: `cargo run -p baqueano-replay --example fake_upstream -- --port 18181`.

use anyhow::Context;

// The integration tests' fake; its test-only helpers are unused here.
#[allow(dead_code, clippy::unwrap_used)]
#[path = "../tests/common/fake_upstream.rs"]
mod fake_upstream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let port: u16 = match args.as_slice() {
        [] => 18181,
        [flag, port] if flag == "--port" => port.parse().context("--port expects 0-65535")?,
        _ => anyhow::bail!("usage: fake_upstream [--port N]"),
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("cannot listen on 127.0.0.1:{port}"))?;
    println!("listening on {}", listener.local_addr()?);
    let fake = std::sync::Arc::new(fake_upstream::Fake::default());
    axum::serve(listener, fake_upstream::router(fake)).await?;
    Ok(())
}
