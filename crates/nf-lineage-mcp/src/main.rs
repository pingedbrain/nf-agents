//! `nf-lineage-mcp`: serve a Nextflow lineage store to AI agents over MCP on stdin/stdout.

mod config;

use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use nf_lineage::ObjectLineageStore;
use nf_lineage_mcp::LineageServer;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    // Usage errors exit 2 (clap); startup and transport errors exit 1. Stdout carries only
    // JSON-RPC, so every log line goes to stderr.
    let cli = config::Cli::parse();
    let filter =
        EnvFilter::try_from_env("NF_LINEAGE_MCP_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Written directly so the reason shows even with logging turned off.
            let _ = writeln!(std::io::stderr(), "nf-lineage-mcp: {e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: config::Cli) -> anyhow::Result<()> {
    let store = ObjectLineageStore::open(&cli.store)
        .with_context(|| format!("cannot open the lineage store {}", cli.store))?;
    tracing::info!(store = %cli.store, "serving the lineage store over stdio (read-only)");
    let server = LineageServer::new(Arc::new(store), cli.settings());
    // Ends on stdin EOF once in-flight calls are answered.
    server
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
