//! `baqueano-replay` command line: parse flags, load the cassette, serve until SIGINT/SIGTERM.

use std::process::ExitCode;

use anyhow::anyhow;
use baqueano_replay::config::{Cli, Config};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    // Usage errors (clap's and ours) exit 2; startup errors exit 1 with one line on stderr.
    let api_key = std::env::var("BAQUEANO_REPLAY_API_KEY").ok();
    let config = match Config::from_cli(Cli::parse(), api_key) {
        Ok(config) => config,
        Err(e) => Cli::command()
            .error(ErrorKind::MissingRequiredArgument, e)
            .exit(),
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    match run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("baqueano-replay: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(config: Config) -> anyhow::Result<()> {
    let app = baqueano_replay::app(&config)?;
    let (bind, port) = (config.bind, config.port);
    let listener = tokio::net::TcpListener::bind((bind, port))
        .await
        .map_err(|e| anyhow!("cannot listen on {bind}:{port}: {e}"))?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

/// Resolves on SIGINT or SIGTERM.
async fn shutdown() {
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        () = terminate => {}
    }
    tracing::info!("shutting down");
}
