//! Record/replay proxy for LLM HTTP APIs, so Nextflow agent pipelines can be tested offline.
//!
//! **Status: pre-alpha, for research and testing use only.** Requests are normalized into a
//! stable key ([`normalize`]) and answered from a reviewable JSONL cassette ([`cassette`]).

pub mod cassette;
pub mod config;
pub mod normalize;
mod proxy;
mod upstream;

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;

use crate::cassette::{Cassette, CassetteError, Recorder};
use crate::config::{Config, ConfigError, Mode};
use crate::normalize::{NormalizeError, Normalizer};
use crate::proxy::{AppState, Backend};
use crate::upstream::{Upstream, UpstreamError};

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
    /// Record and passthrough need `NF_LLM_REPLAY_API_KEY` (non-empty).
    #[error("NF_LLM_REPLAY_API_KEY must be set (non-empty) in {0:?} mode")]
    MissingApiKey(Mode),
    /// The upstream client cannot be built.
    #[error(transparent)]
    Upstream(#[from] UpstreamError),
}

/// Builds the proxy router; loads and validates the cassette first.
pub fn app(config: &Config) -> Result<Router, StartupError> {
    let mode = config.mode;
    let cassette = || {
        config
            .cassette
            .clone()
            .ok_or(ConfigError::MissingCassette(mode))
    };
    let key = || {
        config
            .api_key
            .clone()
            .ok_or(StartupError::MissingApiKey(mode))
    };
    let upstream = || -> Result<Upstream, StartupError> {
        let base = config
            .upstream
            .as_deref()
            .ok_or(ConfigError::MissingUpstream(mode))?;
        Ok(Upstream::new(base, key()?)?)
    };
    let backend = match mode {
        Mode::Replay => Backend::Replay(Cassette::load(&cassette()?)?),
        // Upstream first: a missing key must fail before the cassette file is created.
        Mode::Record => {
            let upstream = upstream()?;
            Backend::Record(Recorder::open(cassette()?, key()?)?, upstream)
        }
        Mode::Passthrough => Backend::Passthrough(upstream()?),
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
