//! Command line flags, environment variables and the validated runtime configuration.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

use axum::http::Uri;

/// How the proxy answers requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    /// Answer missing keys from the upstream and add them to the cassette.
    Record,
    /// Answer only from the cassette; never touch the network.
    Replay,
    /// Forward everything upstream; no cassette.
    Passthrough,
}

/// The upstream provider key; never printed (`Debug` shows `***`).
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    /// The secret value, for the upstream request only.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

/// Command line; every flag can also come from its environment variable.
#[derive(Debug, clap::Parser)]
#[command(
    name = "nf-llm-replay",
    version,
    about = "Record/replay proxy for LLM HTTP APIs (pre-alpha, research and testing use only).",
    after_help = "The upstream key is read only from the NF_LLM_REPLAY_API_KEY environment variable (record and passthrough modes)."
)]
pub struct Cli {
    /// Proxy mode.
    #[arg(long, env = "NF_LLM_REPLAY_MODE", value_enum, default_value = "replay")]
    pub mode: Mode,
    /// JSONL cassette file (required unless --mode passthrough).
    #[arg(long, env = "NF_LLM_REPLAY_CASSETTE")]
    pub cassette: Option<PathBuf>,
    /// Upstream base URL, e.g. https://api.openai.com (record and passthrough).
    #[arg(long, env = "NF_LLM_REPLAY_UPSTREAM", value_parser = parse_upstream)]
    pub upstream: Option<String>,
    /// Port to listen on (0 = ephemeral).
    #[arg(long, env = "NF_LLM_REPLAY_PORT", default_value_t = 8080)]
    pub port: u16,
    /// Address to listen on.
    #[arg(long, env = "NF_LLM_REPLAY_BIND", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub bind: IpAddr,
}

fn parse_upstream(s: &str) -> Result<String, String> {
    let uri: Uri = s.parse().map_err(|e| format!("{e}"))?;
    match (uri.scheme_str(), uri.host()) {
        (Some("http" | "https"), Some(host)) if !host.is_empty() => Ok(s.to_owned()),
        _ => Err("expected an http:// or https:// URL".into()),
    }
}

/// Validated configuration.
#[derive(Debug)]
pub struct Config {
    /// Proxy mode.
    pub mode: Mode,
    /// Cassette path; always set unless passthrough.
    pub cassette: Option<PathBuf>,
    /// Upstream base URL.
    pub upstream: Option<String>,
    /// Listen address.
    pub bind: IpAddr,
    /// Listen port.
    pub port: u16,
    /// Upstream key from `NF_LLM_REPLAY_API_KEY`.
    pub api_key: Option<ApiKey>,
}

/// An invalid flag combination (a usage error, exit code 2).
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Record and replay need a cassette.
    #[error("--cassette is required in {0:?} mode")]
    MissingCassette(Mode),
}

impl Config {
    /// Checks flag combinations; `api_key` is the `NF_LLM_REPLAY_API_KEY` value.
    pub fn from_cli(cli: Cli, api_key: Option<String>) -> Result<Self, ConfigError> {
        if cli.cassette.is_none() && cli.mode != Mode::Passthrough {
            return Err(ConfigError::MissingCassette(cli.mode));
        }
        Ok(Self {
            mode: cli.mode,
            cassette: cli.cassette,
            upstream: cli.upstream,
            bind: cli.bind,
            port: cli.port,
            api_key: api_key.map(ApiKey),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("nf-llm-replay").chain(args.iter().copied()))
    }

    #[test]
    fn debug_output_redacts_api_key() {
        let cli = parse(&["--cassette", "c.jsonl"]).unwrap();
        let config = Config::from_cli(cli, Some("sk-test-SECRET-123".into())).unwrap();
        let shown = format!("{config:?} {:?}", config.api_key);
        assert!(!shown.contains("sk-test-SECRET-123"), "{shown}");
        assert!(shown.contains("***"), "{shown}");
        assert_eq!(config.api_key.unwrap().expose(), "sk-test-SECRET-123");
    }

    #[test]
    fn defaults_and_required_cassette() {
        let config = Config::from_cli(parse(&["--cassette", "c.jsonl"]).unwrap(), None).unwrap();
        assert_eq!(config.mode, Mode::Replay);
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind.to_string(), "127.0.0.1");
        assert!(config.upstream.is_none() && config.api_key.is_none());

        let err = Config::from_cli(parse(&[]).unwrap(), None).unwrap_err();
        assert!(err.to_string().contains("--cassette"), "{err}");
        let pass = parse(&["--mode", "passthrough"]).unwrap();
        assert!(Config::from_cli(pass, None).unwrap().cassette.is_none());
    }

    #[test]
    fn upstream_must_be_an_http_url() {
        for ok in ["http://127.0.0.1:9", "https://api.openai.com"] {
            let cli = parse(&["--cassette", "c", "--upstream", ok]).unwrap();
            assert_eq!(cli.upstream.as_deref(), Some(ok));
        }
        for bad in ["ftp://x", "api.openai.com", "http://", "not a url"] {
            assert!(
                parse(&["--cassette", "c", "--upstream", bad]).is_err(),
                "{bad}"
            );
        }
        assert!(parse(&["--mode", "bogus"]).is_err());
    }
}
