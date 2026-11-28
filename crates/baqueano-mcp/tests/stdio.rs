//! The built binary over stdio: a scripted JSON-RPC session and startup failures.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;

/// Runs the binary with `args` (and `env`), writes `input` to its stdin, closes it, and waits.
fn run(args: &[&str], env: &[(&str, &Path)], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_baqueano-mcp"))
        .args(args)
        .env_remove("BAQUEANO_LINEAGE_STORE")
        .env_remove("BAQUEANO_LINEAGE_MCP_MAX_RESPONSE_BYTES")
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A process that exits early closes the pipe; the write error does not matter then.
    let _ = child.stdin.take().unwrap().write_all(input);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || tx.send(child.wait_with_output()));
    rx.recv_timeout(Duration::from_secs(30))
        .expect("the binary did not exit within 30 s")
        .unwrap()
}

#[test]
fn scripted_session_over_stdio() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let store = root.join("../../fixtures/lineage/26.10.7/.lineage");
    let session = std::fs::read(root.join("tests/data/session.jsonl")).unwrap();
    let out = run(&[], &[("BAQUEANO_LINEAGE_STORE", &store)], &session);
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert!(out.status.success(), "{out:?}");
    let mut responses = std::collections::BTreeMap::new();
    for line in stdout.lines() {
        let message: Value = serde_json::from_str(line).unwrap();
        assert_eq!(message["jsonrpc"], "2.0", "{line}");
        if let Some(id) = message["id"].as_u64() {
            responses.insert(id, message);
        }
    }
    assert_eq!(responses.keys().copied().collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(
        responses[&1]["result"]["serverInfo"]["name"],
        "baqueano-mcp"
    );
    assert_eq!(
        responses[&2]["result"]["tools"].as_array().unwrap().len(),
        6
    );
    let call = &responses[&3]["result"];
    assert_eq!(call["isError"], false, "{call}");
    let runs: Value = serde_json::from_str(call["content"][0]["text"].as_str().unwrap()).unwrap();
    let lids: BTreeSet<&str> = runs
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["lid"].as_str().unwrap())
        .collect();
    assert_eq!(lids.len(), 2, "{runs}");
}

#[test]
fn unopenable_store_fails_fast() {
    let out = run(&["--store", "/nonexistent/x"], &[], b"");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("/nonexistent/x"),
        "{out:?}"
    );
    assert!(out.stdout.is_empty(), "{out:?}");
    // Usage errors exit 2.
    let out = run(&["--store", "/s", "--max-response-bytes", "10"], &[], b"");
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
}
