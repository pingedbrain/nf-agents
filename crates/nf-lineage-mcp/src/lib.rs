//! Read-only [MCP](https://modelcontextprotocol.io) server over a Nextflow data lineage store.
//!
//! **Status: pre-alpha.** [`LineageServer`] exposes [`nf_lineage`] as MCP tools for AI agents:
//! `list_runs`, `view`, `find`, `diff`, `check` and `provenance`. Every tool is read-only and idempotent; lineage and argument
//! failures come back as tool results with `isError: true` and a hint, and results larger than
//! [`Settings::max_response_bytes`] are truncated with a notice. Serve it on any rmcp transport:
//!
//! ```no_run
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use std::sync::Arc;
//! use nf_lineage::ObjectLineageStore;
//! use nf_lineage_mcp::{LineageServer, Settings};
//! use rmcp::ServiceExt;
//!
//! let store = Arc::new(ObjectLineageStore::open("/data/.lineage")?);
//! let settings = Settings {
//!     store_label: "/data/.lineage".into(),
//!     max_response_bytes: 32_768,
//!     check: Default::default(),
//! };
//! let (server_io, _client_io) = tokio::io::duplex(64 * 1024);
//! LineageServer::new(store, settings).serve(server_io).await?.waiting().await?;
//! # Ok(()) }
//! ```

mod respond;
mod server;

pub use server::{DiffArgs, DiffMode, FindArgs, LidArgs, LineageServer, ListRunsArgs, Settings};
