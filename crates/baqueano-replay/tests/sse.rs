//! Server-sent event streams: byte-exact record/replay and incremental passthrough.

// Helpers outside #[test] fns are not covered by clippy.toml's allow-*-in-tests.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;

use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::fake_upstream::{CHAT_EVENTS, MESSAGES_EVENTS, RESPONSES_EVENTS, spawn};
use common::{proxy, replay};
use serde_json::{Value, json};

/// Status, content-type and body bytes of a POST.
async fn fetch(addr: SocketAddr, path: &str, body: &Value) -> (u16, String, Vec<u8>) {
    let reply = reqwest::Client::new()
        .post(format!("http://{addr}{path}"))
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    let ctype = reply.headers()["content-type"].to_str().unwrap().to_owned();
    (
        reply.status().as_u16(),
        ctype,
        reply.bytes().await.unwrap().to_vec(),
    )
}

/// Records `path` through the proxy, replays it offline, and checks both equal the upstream bytes.
async fn record_replay(path: &str, body: Value, events: &[&str]) {
    let (up, fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("sse.jsonl");
    let expected = (
        200,
        "text/event-stream".to_owned(),
        events.concat().into_bytes(),
    );

    let recorded = fetch(proxy("record", &cassette, &up).await, path, &body).await;
    assert_eq!(recorded, expected);
    let replayed = fetch(replay(&cassette).await, path, &body).await;
    assert_eq!(replayed, expected);
    assert_eq!(
        fake.hits.load(Ordering::SeqCst),
        1,
        "replay must stay offline"
    );
}

#[tokio::test]
async fn responses_stream_record_replay_is_byte_exact() {
    let body = json!({"model": "gpt-5-mini", "input": "Is FASTQ binary?", "stream": true});
    record_replay("/v1/responses", body, &RESPONSES_EVENTS).await;
}

#[tokio::test]
async fn messages_stream_with_beta_query_record_replay_is_byte_exact() {
    let body = json!({"model": "claude", "max_tokens": 64, "stream": true,
        "messages": [{"role": "user", "content": "Is FASTQ binary?"}]});
    record_replay("/v1/messages?beta=true", body, &MESSAGES_EVENTS).await;
}

#[tokio::test]
async fn chat_completions_stream_record_replay_is_byte_exact() {
    let body = json!({"model": "gpt-5-mini", "stream": true,
        "messages": [{"role": "user", "content": "Is FASTQ binary?"}]});
    record_replay("/v1/chat/completions", body, &CHAT_EVENTS).await;
}

#[tokio::test]
async fn sse_cassette_stores_one_string_per_event() {
    let (up, _fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("sse.jsonl");
    let rec = proxy("record", &cassette, &up).await;
    fetch(rec, "/v1/responses", &json!({"input": "q", "stream": true})).await;

    let text = std::fs::read_to_string(&cassette).unwrap();
    let entry: Value = serde_json::from_str(text.trim_end()).unwrap();
    let response = &entry["response"];
    assert_eq!(response["events"], json!(RESPONSES_EVENTS));
    assert!(response.get("body").is_none(), "{response}");
    assert_eq!(response["content_type"], "text/event-stream");
}

#[tokio::test]
async fn passthrough_streams_events_incrementally() {
    let (up, fake) = spawn().await;
    fake.pause.store(true, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("absent.jsonl");
    let addr = proxy("passthrough", &cassette, &up).await;

    // The upstream holds event 2 back until resumed: headers and event 1 must already be here.
    let request = reqwest::Client::new()
        .post(format!("http://{addr}/v1/responses"))
        .body(json!({"input": "q", "stream": true}).to_string());
    let (mut reply, first) = tokio::time::timeout(Duration::from_secs(5), async {
        let mut reply = request.send().await.unwrap();
        let first = reply.chunk().await.unwrap().unwrap();
        (reply, first)
    })
    .await
    .expect("event 1 did not arrive while the upstream was paused");
    assert_eq!(first, RESPONSES_EVENTS[0]);

    fake.resume.notify_one();
    let mut body = first.to_vec();
    while let Some(chunk) = reply.chunk().await.unwrap() {
        body.extend_from_slice(&chunk);
    }
    assert_eq!(body, RESPONSES_EVENTS.concat().into_bytes());
    assert!(
        !cassette.exists(),
        "passthrough must not create the cassette"
    );
}
