//! Replay mode end to end: in-process server on an ephemeral port, raw HTTP/1.1 clients.

// Helpers outside #[test] fns are not covered by clippy.toml's allow-*-in-tests.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;

use common::{entry, post, replay, write_cassette};

const CHAT: &str = "/v1/chat/completions";
const Q1: &str = r#"{"model":"gpt-5-mini","messages":[{"role":"user","content":"Is FASTQ binary?"}],"stream":false}"#;
const A1: &str = "{\"id\":\"chatcmpl-fake\",\"choices\":[]}\n";

#[tokio::test]
async fn replay_hit_returns_recorded_status_content_type_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let cassette = write_cassette(
        dir.path(),
        &[
            entry(
                "POST",
                CHAT,
                Q1,
                200,
                Some("application/json; charset=utf-8"),
                A1,
            ),
            entry("GET", "/v1/models", "", 203, None, "plain text"),
        ],
    );
    let addr = replay(&cassette).await;

    let hit = post(addr, CHAT, Q1.as_bytes()).await;
    assert_eq!(hit.status, 200);
    assert_eq!(
        hit.header("content-type"),
        Some("application/json; charset=utf-8")
    );
    assert_eq!(hit.body, A1.as_bytes());

    let get = common::send_raw(addr, b"GET /v1/models?x=1 HTTP/1.1\r\nHost: t\r\n\r\n").await;
    assert_eq!(get.status, 203);
    assert_eq!(get.header("content-type"), None);
    assert_eq!(get.text(), "plain text");
}

#[tokio::test]
async fn replay_repeated_request_returns_identical_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let cassette = write_cassette(
        dir.path(),
        &[entry("POST", CHAT, Q1, 200, Some("application/json"), A1)],
    );
    let addr = replay(&cassette).await;
    // Same request with other key order, whitespace and a volatile field: same key.
    let variant = r#"{ "stream": false, "user": "u-42", "messages":[{"content":"Is FASTQ binary?","role":"user"}], "model":"gpt-5-mini" }"#;

    let first = post(addr, CHAT, Q1.as_bytes()).await;
    let second = post(addr, CHAT, variant.as_bytes()).await;
    let strip = |r: common::Reply| {
        (
            r.status_line,
            r.headers
                .into_iter()
                .filter(|(n, _)| n != "date")
                .collect::<Vec<_>>(),
            r.body,
        )
    };
    assert_eq!(strip(first), strip(second));
}

#[tokio::test]
async fn invalid_json_body_returns_400() {
    let dir = tempfile::tempdir().unwrap();
    let addr = replay(&write_cassette(dir.path(), &[])).await;
    for bad in [&b"{not json"[..], b"\xff\xfe", b"[1,"] {
        let reply = post(addr, CHAT, bad).await;
        assert_eq!(reply.status, 400);
        assert_eq!(reply.header("content-type"), Some("application/json"));
        let error = &reply.json()["error"];
        assert_eq!(error["type"], "nf_llm_replay_bad_request");
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("not valid JSON"),
            "{error}"
        );
    }
}

#[tokio::test]
async fn eight_mib_request_body_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let big = format!(
        r#"{{"model":"m","messages":[{{"role":"user","content":"{}"}}]}}"#,
        "a".repeat(8 * 1024 * 1024)
    );
    let cassette = write_cassette(
        dir.path(),
        &[entry("POST", CHAT, &big, 200, Some("application/json"), A1)],
    );
    let addr = replay(&cassette).await;
    let reply = post(addr, CHAT, big.as_bytes()).await;
    assert_eq!(reply.status, 200);
    assert_eq!(reply.body, A1.as_bytes());
}

#[tokio::test]
async fn replay_miss_returns_599_with_key_summary_and_record_command() {
    let dir = tempfile::tempdir().unwrap();
    let cassette = write_cassette(dir.path(), &[]);
    let addr = replay(&cassette).await;
    let body = r#"{"model":"gpt-5-mini","messages":[{"role":"system","content":"s"},{"role":"user","content":"make this loud: hello"}]}"#;

    let reply = post(addr, CHAT, body.as_bytes()).await;
    assert_eq!(reply.status, 599);
    assert_eq!(reply.header("content-type"), Some("application/json"));
    let json = reply.json();
    let error = &json["error"];
    assert_eq!(error["type"], "nf_llm_replay_miss");
    let key = error["key"].as_str().unwrap();
    assert_eq!(key.len(), 64);
    let request = &error["request"];
    assert_eq!(request["method"], "POST");
    assert_eq!(request["path"], CHAT);
    assert_eq!(request["model"], "gpt-5-mini");
    assert_eq!(request["messages"], 2);
    assert_eq!(request["last_message"], "make this loud: hello");
    let path = cassette.to_str().unwrap();
    assert_eq!(error["cassette"], path);
    let command = error["record_command"].as_str().unwrap();
    assert_eq!(
        command,
        format!(
            "NF_LLM_REPLAY_API_KEY=<key> nf-llm-replay --mode record --cassette {path} --upstream https://api.openai.com"
        )
    );
    let message = error["message"].as_str().unwrap();
    for part in ["POST", CHAT, "gpt-5-mini", &key[..12], command] {
        assert!(message.contains(part), "{message} lacks {part}");
    }
    assert_eq!(message.lines().count(), 1);

    // An explicit --upstream wins over the per-path default.
    let addr = common::start(common::config(&[
        "--cassette",
        path,
        "--upstream",
        "http://127.0.0.1:9",
    ]))
    .await;
    let json = post(addr, "/v1/messages", b"{}").await.json();
    assert!(
        json["error"]["record_command"]
            .as_str()
            .unwrap()
            .ends_with("--upstream http://127.0.0.1:9")
    );
    assert_eq!(json["error"]["request"]["model"], serde_json::Value::Null);
}

#[tokio::test]
async fn replay_never_contacts_upstream() {
    let (upstream, connections) = common::counting_upstream().await;
    let dir = tempfile::tempdir().unwrap();
    let cassette = write_cassette(
        dir.path(),
        &[entry("POST", CHAT, Q1, 200, Some("application/json"), A1)],
    );
    let addr = common::start(common::config(&[
        "--cassette",
        cassette.to_str().unwrap(),
        "--upstream",
        &upstream,
    ]))
    .await;

    assert_eq!(post(addr, CHAT, Q1.as_bytes()).await.status, 200);
    assert_eq!(post(addr, CHAT, br#"{"model":"other"}"#).await.status, 599);
    assert_eq!(
        post(addr, "/v1/messages?beta=true", b"{}").await.status,
        599
    );
    // Prove the counter works, then that replay added nothing to it.
    tokio::net::TcpStream::connect(upstream.trim_start_matches("http://"))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(connections.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[tokio::test]
async fn replay_ignores_h2c_upgrade_and_placeholder_bearer() {
    // Request shape copied from the langchain4j capture (Nextflow 26.10.7, Java HttpClient).
    let body = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/captures/langchain4j-json-schema.json"
    ))
    .unwrap();
    let body = serde_json::from_str::<serde_json::Value>(&body).unwrap()["body"].to_string();
    let dir = tempfile::tempdir().unwrap();
    let cassette = write_cassette(
        dir.path(),
        &[entry(
            "POST",
            CHAT,
            &body,
            200,
            Some("application/json"),
            A1,
        )],
    );
    let addr = replay(&cassette).await;

    let raw = format!(
        "POST {CHAT} HTTP/1.1\r\nConnection: Upgrade, HTTP2-Settings\r\nContent-Length: {}\r\nHost: 127.0.0.1:18080\r\n\
         HTTP2-Settings: AAEAAEAAAAIAAAAAAAMAAAAAAAQBAAAAAAUAAEAAAAYABgAA\r\nUpgrade: h2c\r\n\
         Authorization: Bearer nxf-no-credential\r\nContent-Type: application/json\r\nUser-Agent: langchain4j-openai\r\n\r\n{body}",
        body.len()
    );
    let reply = common::send_raw(addr, raw.as_bytes()).await;
    assert!(
        reply.status_line.starts_with("HTTP/1.1 200"),
        "{}",
        reply.status_line
    );
    assert_eq!(reply.body, A1.as_bytes());
}

#[tokio::test]
async fn replay_serves_64_concurrent_requests() {
    let dir = tempfile::tempdir().unwrap();
    let requests: Vec<(String, String)> = (0..3)
        .map(|i| {
            (
                format!(r#"{{"model":"m","messages":[{{"role":"user","content":"q{i}"}}]}}"#),
                format!("answer {i}"),
            )
        })
        .collect();
    let lines: Vec<String> = requests
        .iter()
        .map(|(q, a)| entry("POST", CHAT, q, 200, Some("text/plain"), a))
        .collect();
    let addr = replay(&write_cassette(dir.path(), &lines)).await;

    let tasks: Vec<_> = (0..64)
        .map(|i| {
            let (q, a) = requests[i % 3].clone();
            tokio::spawn(async move { (post(addr, CHAT, q.as_bytes()).await, a) })
        })
        .collect();
    for task in tasks {
        let (reply, expected) = task.await.unwrap();
        assert_eq!((reply.status, reply.text()), (200, expected));
    }
}
