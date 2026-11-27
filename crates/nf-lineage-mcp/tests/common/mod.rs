//! Fixture and MCP client helpers shared by the integration tests.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nf_lineage::ObjectLineageStore;
use nf_lineage_mcp::{LineageServer, Settings};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::{RunningService, ServiceError};
use rmcp::{RoleClient, ServiceExt};
use serde_json::Value;

pub type Client = RunningService<RoleClient, ()>;

/// `fixtures/lineage` at the repository root.
pub fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/lineage")
}

/// The core fixture versions (`manifest.json` kind `core`), sorted.
pub fn core_versions() -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(fixture_root())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|dir| {
            let manifest: Value =
                serde_json::from_slice(&fs::read(dir.join("manifest.json")).unwrap()).unwrap();
            manifest["kind"] == "core"
        })
        .map(|dir| dir.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    out.sort();
    assert!(out.len() >= 4, "core fixtures missing: {out:?}");
    out
}

/// `oracle/ids.json` of a fixture version (`RUN1`, `RUN2`, `TASK_<P>_RUN<n>` → LID).
pub fn ids(version: &str) -> BTreeMap<String, String> {
    serde_json::from_slice(&fs::read(fixture_root().join(version).join("oracle/ids.json")).unwrap())
        .unwrap()
}

/// The stdout lines of an oracle case (`oracle/<case>.out`).
pub fn oracle_lines(version: &str, case: &str) -> Vec<String> {
    fs::read_to_string(
        fixture_root()
            .join(version)
            .join(format!("oracle/{case}.out")),
    )
    .unwrap()
    .lines()
    .map(str::to_owned)
    .collect()
}

/// A client connected over an in-memory duplex pipe to a server on `version`'s store.
pub async fn start(version: &str, max_response_bytes: usize) -> Client {
    let dir = fixture_root().join(version).join(".lineage");
    let store = Arc::new(ObjectLineageStore::open(dir.to_str().unwrap()).unwrap());
    let settings = Settings {
        store_label: dir.display().to_string(),
        max_response_bytes,
    };
    let (server_io, client_io) = tokio::io::duplex(1 << 20);
    tokio::spawn(async move {
        if let Ok(running) = LineageServer::new(store, settings).serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    ().serve(client_io).await.unwrap()
}

/// Calls `tool` with JSON object `args` and returns the raw outcome.
pub async fn try_call(
    client: &Client,
    tool: &str,
    args: Value,
) -> Result<CallToolResult, ServiceError> {
    let params = CallToolRequestParams::new(tool.to_owned())
        .with_arguments(args.as_object().cloned().unwrap());
    client.call_tool(params).await
}

/// Calls `tool` and returns `(is_error, text blocks)`.
pub async fn call(client: &Client, tool: &str, args: Value) -> (bool, Vec<String>) {
    let result = try_call(client, tool, args).await.unwrap();
    let texts = result
        .content
        .iter()
        .map(|c| c.as_text().unwrap().text.clone())
        .collect();
    (result.is_error == Some(true), texts)
}

/// Calls a tool that must succeed with a single, untruncated JSON block.
pub async fn call_json(client: &Client, tool: &str, args: Value) -> Value {
    let (is_error, texts) = call(client, tool, args).await;
    assert!(!is_error, "{tool} failed: {texts:?}");
    assert_eq!(texts.len(), 1, "{texts:?}");
    serde_json::from_str(&texts[0]).unwrap()
}
