//! Recorded cassettes never contain credentials, credential headers or query strings.

// Helpers outside #[test] fns are not covered by clippy.toml's allow-*-in-tests.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;

use common::fake_upstream::spawn;
use common::{KEY, post_with, proxy};

#[tokio::test]
async fn recorded_cassette_contains_no_credentials() {
    let (up, _fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("scan.jsonl");
    let rec = proxy("record", &cassette, &up).await;
    let sessions = [
        (
            "/v1/chat/completions",
            "Authorization: Bearer sk-client-bearer\r\n",
        ),
        (
            "/v1/messages?beta=true",
            "x-api-key: sk-client-anthropic\r\n",
        ),
        (
            "/v1/models/x:generate?key=sk-client-query",
            "x-goog-api-key: sk-client-goog\r\n",
        ),
    ];
    for (i, (path, headers)) in sessions.iter().enumerate() {
        let body = format!(r#"{{"model":"m","messages":[{{"role":"user","content":"q{i}"}}]}}"#);
        assert_eq!(
            post_with(rec, path, headers, body.as_bytes()).await.status,
            200
        );
    }

    let text = std::fs::read_to_string(&cassette).unwrap();
    assert_eq!(text.lines().count(), 3);
    for needle in [
        KEY,
        "sk-client",
        "authorization",
        "Authorization",
        "x-api-key",
        "x-goog-api-key",
        "Bearer ",
        "key=",
        "beta=true",
    ] {
        assert!(
            !text.contains(needle),
            "cassette contains `{needle}`:\n{text}"
        );
    }
}
