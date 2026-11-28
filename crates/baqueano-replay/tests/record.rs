//! Record and passthrough modes end to end against the in-process fake upstream.

// Helpers outside #[test] fns are not covered by clippy.toml's allow-*-in-tests.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;

use std::path::Path;
use std::sync::atomic::Ordering;

use baqueano_replay::cassette::Cassette;
use common::fake_upstream::spawn;
use common::{KEY, Reply, entry, post, post_with, proxy, replay, write_cassette};
use serde_json::{Value, json};

const CHAT: &str = "/v1/chat/completions";

fn chat(content: &str) -> Vec<u8> {
    json!({"model": "gpt-5-mini", "messages": [{"role": "user", "content": content}]})
        .to_string()
        .into_bytes()
}

/// What a client sees (status, content-type, body).
fn seen(r: &Reply) -> (u16, Option<String>, Vec<u8>) {
    (
        r.status,
        r.header("content-type").map(Into::into),
        r.body.clone(),
    )
}

fn lines(cassette: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(cassette).unwrap_or_default();
    text.lines().map(Into::into).collect()
}

fn content(r: &Reply) -> Value {
    r.json()["choices"][0]["message"].clone()
}

#[tokio::test]
async fn record_then_replay_round_trip_chat_completions() {
    let (up, fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("new.jsonl");
    let rec = proxy("record", &cassette, &up).await;
    let plain = chat("Is FASTQ binary?");
    let schema = json!({"model": "gpt-5-mini", "messages": [{"role": "user", "content": "Is FASTQ binary?"}],
        "response_format": {"type": "json_schema", "json_schema": {"name": "a", "strict": true, "schema": {}}}})
    .to_string();

    let recorded = [
        post(rec, CHAT, &plain).await,
        post(rec, CHAT, schema.as_bytes()).await,
    ];
    assert_eq!(content(&recorded[0])["content"], "FASTQ is a text format.");
    assert_eq!(
        content(&recorded[1])["content"],
        r#"{"answer":"no","confidence":0.9}"#
    );
    assert_eq!(recorded[0].header("content-type"), Some("application/json"));
    assert_eq!(
        (fake.hits.load(Ordering::SeqCst), lines(&cassette).len()),
        (2, 2)
    );

    let off = replay(&cassette).await;
    let replayed = [
        post(off, CHAT, &plain).await,
        post(off, CHAT, schema.as_bytes()).await,
    ];
    assert_eq!(seen(&replayed[0]), seen(&recorded[0]));
    assert_eq!(seen(&replayed[1]), seen(&recorded[1]));
    assert_eq!(
        fake.hits.load(Ordering::SeqCst),
        2,
        "replay must stay offline"
    );
}

#[tokio::test]
async fn multi_turn_tool_round_trip_replays_offline() {
    let (up, _fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("tools.jsonl");
    let rec = proxy("record", &cassette, &up).await;
    let tools = json!([{"type": "function", "function": {"name": "uppercase",
        "parameters": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}}}]);
    let user = json!({"role": "user", "content": "make this loud: hello\nthanks"});
    let first = json!({"model": "gpt-5-mini", "messages": [user], "tools": tools}).to_string();

    let call = post(rec, CHAT, first.as_bytes()).await;
    let message = content(&call);
    assert_eq!(message["tool_calls"][0]["id"], "call_1");
    assert_eq!(message["tool_calls"][0]["function"]["name"], "uppercase");
    assert_eq!(
        message["tool_calls"][0]["function"]["arguments"],
        r#"{"text":"hello"}"#
    );
    let tool = json!({"role": "tool", "tool_call_id": "call_1", "content": "HELLO"});
    let second = json!({"model": "gpt-5-mini", "messages": [user, message, tool], "tools": tools})
        .to_string();
    let answer = post(rec, CHAT, second.as_bytes()).await;
    assert_eq!(content(&answer)["content"], "HELLO");

    let off = replay(&cassette).await;
    assert_eq!(seen(&post(off, CHAT, first.as_bytes()).await), seen(&call));
    assert_eq!(
        seen(&post(off, CHAT, second.as_bytes()).await),
        seen(&answer)
    );
}

#[tokio::test]
async fn concurrent_identical_requests_call_upstream_once() {
    let (up, fake) = spawn().await;
    fake.delay_ms.store(200, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("once.jsonl");
    let rec = proxy("record", &cassette, &up).await;
    let body = chat("Is FASTQ binary?");

    let tasks: Vec<_> = (0..16)
        .map(|_| {
            let body = body.clone();
            tokio::spawn(async move { seen(&post(rec, CHAT, &body).await) })
        })
        .collect();
    let mut replies = Vec::new();
    for task in tasks {
        replies.push(task.await.unwrap());
    }
    replies.push(seen(&post(rec, CHAT, &body).await));
    assert_eq!(fake.hits.load(Ordering::SeqCst), 1);
    assert_eq!(replies[0].0, 200);
    assert!(
        replies.iter().all(|r| *r == replies[0]),
        "{} replies differ",
        replies.len()
    );
    assert_eq!(lines(&cassette).len(), 1);
}

#[tokio::test]
async fn existing_key_is_served_without_upstream_call() {
    let (up, fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let old = entry(
        "POST",
        CHAT,
        &String::from_utf8(chat("old")).unwrap(),
        200,
        Some("application/json"),
        "kept",
    );
    let cassette = write_cassette(dir.path(), std::slice::from_ref(&old));
    let rec = proxy("record", &cassette, &up).await;

    let known = post(rec, CHAT, &chat("old")).await;
    assert_eq!((known.status, known.text()), (200, "kept".to_owned()));
    assert_eq!(fake.hits.load(Ordering::SeqCst), 0);

    assert_eq!(post(rec, CHAT, &chat("new")).await.status, 200);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 1);
    let now = lines(&cassette);
    assert_eq!(now.len(), 2);
    assert!(
        now.contains(&old),
        "existing line must be kept byte-for-byte"
    );
}

#[tokio::test]
async fn non_2xx_upstream_response_is_returned_but_not_recorded() {
    let (up, fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("err.jsonl");
    let rec = proxy("record", &cassette, &up).await;
    let body = chat("Is FASTQ binary?");

    fake.status.store(429, Ordering::SeqCst);
    let limited = post(rec, CHAT, &body).await;
    assert_eq!(limited.status, 429);
    assert_eq!(limited.text(), r#"{"error":{"message":"forced"}}"#);
    assert!(lines(&cassette).is_empty());

    // The failed attempt does not stick: the next request records normally.
    fake.status.store(0, Ordering::SeqCst);
    assert_eq!(post(rec, CHAT, &body).await.status, 200);
    assert_eq!(
        (fake.hits.load(Ordering::SeqCst), lines(&cassette).len()),
        (2, 1)
    );
}

#[tokio::test]
async fn echoed_key_returns_500_and_is_not_recorded() {
    let (up, fake) = spawn().await;
    fake.echo_key.store(true, Ordering::SeqCst);
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("echo.jsonl");
    let rec = proxy("record", &cassette, &up).await;

    let reply = post(rec, CHAT, &chat("leak it")).await;
    assert_eq!(reply.status, 500);
    assert_eq!(
        reply.json()["error"]["type"],
        "baqueano_replay_cassette_error"
    );
    assert!(!reply.text().contains(KEY), "{}", reply.text());
    assert!(lines(&cassette).is_empty());
}

#[tokio::test]
async fn env_key_is_injected_and_client_credentials_are_stripped() {
    let (up, fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let rec = proxy("record", &dir.path().join("auth.jsonl"), &up).await;
    let h2c = "Connection: Upgrade, HTTP2-Settings\r\nUpgrade: h2c\r\nHTTP2-Settings: AAMAAABkAAQAAP__\r\n";
    let headers = format!("Authorization: Bearer nxf-no-credential\r\n{h2c}");

    assert_eq!(post_with(rec, CHAT, &headers, &chat("q")).await.status, 200);
    let sent = fake.take_last().unwrap().headers;
    assert_eq!(sent["authorization"], format!("Bearer {KEY}").as_str());
    for name in ["upgrade", "http2-settings", "x-api-key"] {
        assert!(!sent.contains_key(name), "{name} forwarded: {sent:?}");
    }
    assert!(!format!("{sent:?}").contains("nxf-no-credential"));
}

#[tokio::test]
async fn query_string_and_provider_headers_are_forwarded() {
    let (up, fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let rec = proxy("record", &dir.path().join("fwd.jsonl"), &up).await;
    let headers = "x-api-key: dummy\r\nanthropic-version: 2023-06-01\r\n";
    let body = br#"{"model":"claude","max_tokens":5,"messages":[{"role":"user","content":"hi"}]}"#;

    assert_eq!(
        post_with(rec, "/v1/messages?beta=true", headers, body)
            .await
            .status,
        200
    );
    let seen = fake.take_last().unwrap();
    assert_eq!(seen.uri, "/v1/messages?beta=true");
    assert_eq!(seen.body, &body[..], "original body bytes are forwarded");
    assert_eq!(seen.headers["x-api-key"], KEY);
    assert_eq!(seen.headers["anthropic-version"], "2023-06-01");
    assert!(!seen.headers.contains_key("authorization"));
}

#[tokio::test]
async fn unreachable_upstream_returns_502_json() {
    let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("down.jsonl");
    let rec = proxy("record", &cassette, &up).await;

    let reply = post(rec, CHAT, &chat("q")).await;
    assert_eq!(reply.status, 502);
    assert_eq!(reply.header("content-type"), Some("application/json"));
    assert_eq!(
        reply.json()["error"]["type"],
        "baqueano_replay_upstream_error"
    );
    assert!(lines(&cassette).is_empty());
}

#[tokio::test]
async fn passthrough_forwards_without_touching_the_cassette() {
    let (up, fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("never.jsonl");
    let pass = proxy("passthrough", &cassette, &up).await;

    let reply = post(pass, "/v1/chat/completions?x=1", &chat("q")).await;
    assert_eq!(content(&reply)["content"], "FASTQ is a text format.");
    assert_eq!(
        fake.take_last().unwrap().headers["authorization"],
        format!("Bearer {KEY}").as_str()
    );
    // Non-JSON bodies are forwarded unchanged; the upstream's own error comes back.
    let odd = post(pass, CHAT, b"not json").await;
    assert_eq!(
        (odd.status, odd.text()),
        (400, "fake upstream: body is not JSON".to_owned())
    );
    assert_eq!(fake.take_last().unwrap().body, &b"not json"[..]);
    assert_eq!(fake.hits.load(Ordering::SeqCst), 2);
    assert!(
        !cassette.exists(),
        "passthrough must not create the cassette"
    );
}

#[tokio::test]
async fn record_concurrent_distinct_requests_produce_valid_sorted_cassette() {
    let (up, fake) = spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = dir.path().join("conc.jsonl");
    let rec = proxy("record", &cassette, &up).await;

    let tasks: Vec<_> = (0..32)
        .map(|i| tokio::spawn(async move { post(rec, CHAT, &chat(&format!("q{i}"))).await.status }))
        .collect();
    for task in tasks {
        assert_eq!(task.await.unwrap(), 200);
    }
    assert_eq!(fake.hits.load(Ordering::SeqCst), 32);
    let keys: Vec<String> = lines(&cassette)
        .iter()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["key"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!((keys.len(), &keys), (32, &sorted));
    assert!(Cassette::load(&cassette).is_ok());
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        1,
        "no temp files left"
    );
}
