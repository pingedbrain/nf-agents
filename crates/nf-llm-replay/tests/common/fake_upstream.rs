//! Deterministic fake LLM provider (chat completions JSON) with knobs and observations for tests.

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

/// What the fake saw and how it misbehaves (all knobs off by default).
#[derive(Default)]
pub struct Fake {
    /// Requests received.
    pub hits: AtomicUsize,
    /// The last request.
    pub last: Mutex<Option<Seen>>,
    /// Answer every request with this status (0 = normal answers).
    pub status: AtomicU16,
    /// Wait this long before answering.
    pub delay_ms: AtomicU64,
    /// Put the received credential in the answer (a provider leaking the key).
    pub echo_key: AtomicBool,
}

/// One received request.
pub struct Seen {
    /// Path and query.
    pub uri: String,
    /// Request headers.
    pub headers: HeaderMap,
    /// Request body bytes.
    pub body: Bytes,
}

impl Fake {
    /// The last request (`None` before the first one).
    pub fn take_last(&self) -> Option<Seen> {
        self.last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// The fake as a router (the offline example serves it on a fixed port).
pub fn router(fake: Arc<Fake>) -> Router {
    Router::new().fallback(handle).with_state(fake)
}

/// Serves a fresh fake on an ephemeral local port; returns its base URL.
pub async fn spawn() -> (String, Arc<Fake>) {
    let fake = Arc::new(Fake::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = router(fake.clone());
    tokio::spawn(async move { axum::serve(listener, app).await });
    (url, fake)
}

async fn handle(
    State(fake): State<Arc<Fake>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    let credential = ["authorization", "x-api-key", "api-key"]
        .iter()
        .find_map(|name| headers.get(*name)?.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let request: Option<Value> = serde_json::from_slice(&body).ok();
    *fake.last.lock().unwrap_or_else(PoisonError::into_inner) = Some(Seen {
        uri: uri.to_string(),
        headers,
        body,
    });
    let delay = fake.delay_ms.load(Ordering::SeqCst);
    if delay > 0 {
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }
    let json_type = [(header::CONTENT_TYPE, "application/json")];
    let forced = fake.status.load(Ordering::SeqCst);
    if forced != 0 {
        let status = StatusCode::from_u16(forced).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        return (status, json_type, r#"{"error":{"message":"forced"}}"#).into_response();
    }
    let Some(request) = request else {
        return (StatusCode::BAD_REQUEST, "fake upstream: body is not JSON").into_response();
    };
    let mut message = reply(&request);
    if fake.echo_key.load(Ordering::SeqCst) {
        message = json!({"role": "assistant", "content": format!("you sent {credential}")});
    }
    let finish = if message.get("tool_calls").is_some() {
        "tool_calls"
    } else {
        "stop"
    };
    let body = json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion",
        "created": 0,
        "model": request.get("model").cloned().unwrap_or(json!("fake")),
        "choices": [{"index": 0, "message": message, "finish_reason": finish}],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
    });
    (json_type, body.to_string()).into_response()
}

/// The assistant message: echo the last tool result, call `tools[0]`, answer a schema, or chat.
fn reply(request: &Value) -> Value {
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let role = |m: &&Value, r: &str| m.get("role").and_then(Value::as_str) == Some(r);
    if let Some(tool) = messages.iter().rev().find(|m| role(m, "tool")) {
        return json!({"role": "assistant", "content": tool.get("content")});
    }
    if let Some(function) = request.pointer("/tools/0/function") {
        let param = function
            .pointer("/parameters/required/0")
            .and_then(Value::as_str)
            .unwrap_or("input");
        // Text after the last ": " of the first line of the last user message.
        let text = messages
            .iter()
            .rev()
            .find(|m| role(m, "user"))
            .and_then(|m| m.get("content")?.as_str());
        let line = text.unwrap_or_default().lines().next().unwrap_or_default();
        let arg = line.rsplit_once(": ").map_or(line, |(_, after)| after);
        let arguments = json!({ param: arg }).to_string();
        return json!({"role": "assistant", "content": null, "tool_calls": [{
            "id": "call_1", "type": "function",
            "function": {"name": function.get("name"), "arguments": arguments},
        }]});
    }
    if request
        .pointer("/response_format/type")
        .and_then(Value::as_str)
        == Some("json_schema")
    {
        return json!({"role": "assistant", "content": r#"{"answer":"no","confidence":0.9}"#});
    }
    json!({"role": "assistant", "content": "FASTQ is a text format."})
}
