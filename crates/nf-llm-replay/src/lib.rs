//! Record/replay proxy for LLM HTTP APIs, so Nextflow agent pipelines can be tested offline.
//!
//! **Status: pre-alpha, for research and testing use only.** Requests are normalized into a
//! stable key ([`normalize`]) and answered from a reviewable JSONL cassette ([`cassette`]).

pub mod cassette;
pub mod config;
pub mod normalize;
mod proxy;

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;

use crate::cassette::{Cassette, CassetteError};
use crate::config::{Config, ConfigError, Mode};
use crate::normalize::{NormalizeError, Normalizer};
use crate::proxy::{AppState, Backend};

/// Largest accepted request body.
const BODY_LIMIT: usize = 32 * 1024 * 1024;

/// Why the proxy cannot start.
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    /// Invalid flag combination.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// The cassette is missing or invalid.
    #[error(transparent)]
    Cassette(#[from] CassetteError),
    /// A volatile pattern does not compile.
    #[error(transparent)]
    Normalize(#[from] NormalizeError),
    /// Mode not available in this build yet.
    #[error("mode {0:?} is not implemented yet")]
    Unsupported(Mode),
}

/// Builds the proxy router; loads and validates the cassette first.
pub fn app(config: &Config) -> Result<Router, StartupError> {
    let backend = match config.mode {
        Mode::Replay => {
            let path = config
                .cassette
                .as_deref()
                .ok_or(ConfigError::MissingCassette(config.mode))?;
            Backend::Replay(Arc::new(Cassette::load(path)?))
        }
        mode => return Err(StartupError::Unsupported(mode)),
    };
    let state = AppState {
        normalizer: Normalizer::new()?,
        backend,
        cassette: config
            .cassette
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        upstream: config.upstream.clone(),
    };
    Ok(Router::new()
        .fallback(proxy::handle)
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(Arc::new(state)))
}
