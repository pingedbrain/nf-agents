//! Command line of the `baqueano-mcp` binary.

use std::path::PathBuf;

use baqueano_lineage::CheckOptions;
use baqueano_mcp::Settings;

/// Read-only MCP server (stdio) that lets AI agents query a Nextflow data lineage store.
#[derive(Debug, clap::Parser)]
#[command(name = "baqueano-mcp", version, about)]
pub struct Cli {
    /// Lineage store: local path, file://, s3://, gs:// or az:// URL (cloud credentials come from
    /// the standard environment, e.g. AWS_PROFILE).
    #[arg(long, env = "BAQUEANO_LINEAGE_STORE")]
    pub store: String,
    /// Upper bound of one tool result in bytes (at least 1024); larger results are truncated.
    #[arg(
        long,
        env = "BAQUEANO_LINEAGE_MCP_MAX_RESPONSE_BYTES",
        default_value_t = 32_768,
        value_parser = clap::value_parser!(u32).range(1024..)
    )]
    pub max_response_bytes: u32,
    /// For `check`: read data files recorded under FROM below the local directory TO instead
    /// (repeatable; the first matching prefix wins).
    #[arg(long = "path-map", value_name = "FROM=TO", value_parser = parse_path_map)]
    pub path_map: Vec<(String, PathBuf)>,
}

impl Cli {
    /// The server settings: only the operator's flags, nothing a tool call can change.
    pub fn settings(&self) -> Settings {
        Settings {
            store_label: self.store.clone(),
            max_response_bytes: self.max_response_bytes as usize,
            check: CheckOptions {
                path_map: self.path_map.clone(),
                ..CheckOptions::default()
            },
        }
    }
}

fn parse_path_map(value: &str) -> Result<(String, PathBuf), String> {
    match value.split_once('=') {
        Some((from, to)) if !from.is_empty() && !to.is_empty() => Ok((from.to_owned(), to.into())),
        _ => Err(format!(
            "expected FROM=TO with both sides non-empty, got '{value}'"
        )),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use clap::Parser;

    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("baqueano-mcp").chain(args.iter().copied()))
    }

    #[test]
    fn rejects_invalid_flags() {
        for bad in [
            &["--store", "/s", "--max-response-bytes", "10"][..],
            &["--store", "/s", "--max-response-bytes", "1023"],
            &["--store", "/s", "--path-map", "foo"],
            &["--store", "/s", "--path-map", "=x"],
            &["--store", "/s", "--path-map", "/a="],
        ] {
            assert!(parse(bad).is_err(), "{bad:?} accepted");
        }
        if std::env::var_os("BAQUEANO_LINEAGE_STORE").is_none() {
            assert!(parse(&[]).is_err(), "no store accepted");
        }
        let cli = parse(&[
            "--store",
            "/s",
            "--max-response-bytes",
            "1024",
            "--path-map",
            "/a=/b",
            "--path-map",
            "/c=/d",
        ])
        .unwrap();
        let settings = cli.settings();
        assert_eq!(settings.store_label, "/s");
        assert_eq!(settings.max_response_bytes, 1024);
        let map = [
            ("/a".to_owned(), "/b".into()),
            ("/c".to_owned(), "/d".into()),
        ];
        assert_eq!(settings.check.path_map, map);
    }
}
