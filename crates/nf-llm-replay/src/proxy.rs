//! The catch-all request handler and its error responses.

use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::{Value, json};

use crate::cassette::{Cassette, Entry, RecordedRequest, RecordedResponse, Recorder, VERSION};
use crate::config::Mode;
use crate::normalize::{Normalized, Normalizer};
use crate::sse::split_events;
use crate::upstream::Upstream;

/// Shared state of the running proxy.
pub(crate) struct AppState {
    pub(crate) normalizer: Normalizer,
    pub(crate) backend: Backend,
    /// Cassette path as given, for the miss hint.
    pub(crate) cassette: String,
    /// `--upstream`, for the miss hint.
    pub(crate) upstream: Option<String>,
}

/// Where answers come from.
pub(crate) enum Backend {
    Replay(Cassette),
    Record(Recorder, Upstream),
    Passthrough(Upstream),
}

impl Backend {
    fn mode(&self) -> Mode {
        match self {
            Self::Replay(_) => Mode::Replay,
            Self::Record(..) => Mode::Record,
            Self::Passthrough(_) => Mode::Passthrough,
        }
    }
}

/// A failure answered with `{"error":{"type":"nf_llm_replay_<kind>","message":..}}`.
#[derive(Debug)]
pub(crate) enum ProxyError {
    /// 400: the body is not JSON.
    BadRequest(String),
    /// 599: replay has no entry for the key.
    Miss(Box<MissBody>),
    /// 502: the upstream cannot be reached or answered badly.
    Upstream(String),
    /// 500: the entry cannot be stored.
    Cassette(String),
    /// A non-2xx upstream answer, returned unchanged and not recorded.
    NotStored(Box<Response>),
}

impl ProxyError {
    fn outcome(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::Miss(_) => "miss",
            Self::Upstream(_) => "upstream_error",
            Self::Cassette(_) => "cassette_error",
            Self::NotStored(_) => "not_stored",
        }
    }
}

impl From<reqwest::Error> for ProxyError {
    fn from(e: reqwest::Error) -> Self {
        // The URL may carry the client's query (and its credential): never shown.
        let e = e.without_url();
        let mut message = format!("upstream request failed: {e}");
        let mut source = std::error::Error::source(&e);
        while let Some(cause) = source {
            message += &format!(": {cause}");
            source = cause.source();
        }
        Self::Upstream(message)
    }
}

/// What a replay miss reports, so the missing request can be found and recorded.
#[derive(Debug, Serialize)]
pub(crate) struct MissBody {
    #[serde(rename = "type")]
    kind: &'static str,
    message: String,
    key: String,
    request: MissRequest,
    cassette: String,
    record_command: String,
}

/// Summary of the missing request (never the full body).
#[derive(Debug, Serialize)]
pub(crate) struct MissRequest {
    method: String,
    path: String,
    model: Option<String>,
    messages: Option<usize>,
    last_message: Option<String>,
}

/// Longest `last_message` excerpt, in characters.
const EXCERPT_CHARS: usize = 120;

impl MissBody {
    fn new(state: &AppState, key: String, method: &str, path: &str, body: &[u8]) -> Self {
        let request = summarize(method, path, body);
        let upstream = state.upstream.as_deref().unwrap_or(default_upstream(path));
        let record_command = format!(
            "NF_LLM_REPLAY_API_KEY=<key> nf-llm-replay --mode record --cassette {} --upstream {upstream}",
            state.cassette
        );
        let message = format!(
            "no cassette entry for {method} {path} (model {}, key {}); record it with: {record_command}",
            request.model.as_deref().unwrap_or("none"),
            key.get(..12).unwrap_or_default(),
        );
        Self {
            kind: "nf_llm_replay_miss",
            message,
            key,
            request,
            cassette: state.cassette.clone(),
            record_command,
        }
    }
}

fn summarize(method: &str, path: &str, body: &[u8]) -> MissRequest {
    let json: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let model = json.get("model").and_then(Value::as_str).map(str::to_owned);
    let (messages, last) = match (json.get("messages"), json.get("input")) {
        (Some(Value::Array(items)), _) | (_, Some(Value::Array(items))) => {
            (Some(items.len()), items.last())
        }
        (_, Some(input @ Value::String(_))) => (Some(1), Some(input)),
        _ => (None, None),
    };
    let last_message = last.and_then(message_text).map(|text| {
        let mut chars = text.chars();
        let excerpt: String = chars.by_ref().take(EXCERPT_CHARS).collect();
        if chars.next().is_some() {
            excerpt + "…"
        } else {
            excerpt
        }
    });
    MissRequest {
        method: method.into(),
        path: path.into(),
        model,
        messages,
        last_message,
    }
}

/// Text of one message/input item: a string, a string `content`, or its `text` parts joined.
fn message_text(item: &Value) -> Option<String> {
    let content = match item {
        Value::String(s) => return Some(s.clone()),
        other => other.get("content")?,
    };
    match content {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) => {
            let texts: Vec<&str> = parts
                .iter()
                .filter_map(|p| p.get("text")?.as_str())
                .collect();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        _ => None,
    }
}

/// The provider a missing request most likely came from.
fn default_upstream(path: &str) -> &'static str {
    if path.ends_with("/messages") {
        "https://api.anthropic.com"
    } else {
        "https://api.openai.com"
    }
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                json!({"type": "nf_llm_replay_bad_request", "message": message}),
            ),
            Self::Miss(miss) => (
                StatusCode::from_u16(599).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                json!(miss),
            ),
            Self::Upstream(message) => (
                StatusCode::BAD_GATEWAY,
                json!({"type": "nf_llm_replay_upstream_error", "message": message}),
            ),
            Self::Cassette(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"type": "nf_llm_replay_cassette_error", "message": message}),
            ),
            Self::NotStored(response) => return *response,
        };
        let body = json!({ "error": error }).to_string();
        (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
    }
}

/// Serves every method and path.
pub(crate) async fn handle(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let started = Instant::now();
    let mut key = String::new();
    let (outcome, response) = match answer(&state, &method, &uri, &headers, body, &mut key).await {
        Ok(answered) => answered,
        Err(e) => {
            if let ProxyError::Upstream(m) | ProxyError::Cassette(m) = &e {
                tracing::warn!("{m}");
            }
            (e.outcome(), e.into_response())
        }
    };
    tracing::info!(
        mode = ?state.backend.mode(),
        %method,
        path = uri.path(),
        key = key.get(..12).unwrap_or_default(),
        outcome,
        status = response.status().as_u16(),
        ms = started.elapsed().as_millis() as u64,
        "request"
    );
    response
}

/// The response and its log outcome; `key` is set once the request is normalized.
async fn answer(
    state: &AppState,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: Bytes,
    key: &mut String,
) -> Result<(&'static str, Response), ProxyError> {
    let path = uri.path();
    let path_and_query = uri.path_and_query().map_or(path, |pq| pq.as_str());
    let normalize = |key: &mut String| {
        let n = state.normalizer.normalize(method.as_str(), path, &body);
        let n = n.map_err(|e| ProxyError::BadRequest(e.to_string()))?;
        key.clone_from(&n.key);
        Ok::<_, ProxyError>(n)
    };
    match &state.backend {
        Backend::Passthrough(upstream) => {
            let reply = upstream
                .send(method.clone(), path_and_query, headers, body)
                .await?;
            Ok(("forwarded", relay(reply)))
        }
        Backend::Replay(cassette) => {
            let n = normalize(key)?;
            let entry = cassette.get(&n.key).ok_or_else(|| {
                let miss = MissBody::new(state, n.key, method.as_str(), path, &body);
                tracing::warn!("{}", miss.message);
                ProxyError::Miss(Box::new(miss))
            })?;
            Ok(("hit", respond(entry)))
        }
        Backend::Record(recorder, upstream) => {
            let n = normalize(key)?;
            let cell = recorder.cell(&n.key);
            // Only the request whose call filled the cell recorded it; the others are hits.
            let fetched = AtomicBool::new(false);
            let fetch = upstream.send(method.clone(), path_and_query, headers, body.clone());
            let entry = cell
                .get_or_try_init(|| async {
                    fetched.store(true, Ordering::Relaxed);
                    record(recorder, fetch.await?, method, path, n).await
                })
                .await?;
            let outcome = if fetched.load(Ordering::Relaxed) {
                "recorded"
            } else {
                "hit"
            };
            Ok((outcome, respond(entry.clone())))
        }
    }
}

/// Stores a 2xx upstream answer; anything else is handed back unchanged.
async fn record(
    recorder: &Recorder,
    reply: reqwest::Response,
    method: &Method,
    path: &str,
    n: Normalized,
) -> Result<Arc<Entry>, ProxyError> {
    if !reply.status().is_success() {
        return Err(ProxyError::NotStored(Box::new(relay(reply))));
    }
    let status = reply.status().as_u16();
    let content_type = reply
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = String::from_utf8(reply.bytes().await?.into())
        .map_err(|_| ProxyError::Upstream("upstream response body is not UTF-8".into()))?;
    let stream = content_type
        .as_deref()
        .is_some_and(|c| c.starts_with("text/event-stream"));
    let (body, events) = if stream {
        (None, Some(split_events(&body)))
    } else {
        (Some(body), None)
    };
    let canonical = (!n.canonical.is_empty())
        .then(|| RawValue::from_string(n.canonical))
        .transpose()
        .map_err(|e| ProxyError::Cassette(e.to_string()))?;
    let entry = Arc::new(Entry {
        v: VERSION,
        key: n.key,
        request: RecordedRequest {
            method: method.to_string(),
            path: path.into(),
            body: canonical,
        },
        response: RecordedResponse {
            status,
            content_type,
            body,
            events,
        },
    });
    recorder
        .persist(entry.clone())
        .await
        .map_err(|e| ProxyError::Cassette(e.to_string()))?;
    Ok(entry)
}

/// The upstream answer as is, streamed as it arrives, minus hop-by-hop headers.
fn relay(reply: reqwest::Response) -> Response {
    let status = reply.status();
    let mut headers = reply.headers().clone();
    for name in [
        header::CONNECTION,
        header::TRANSFER_ENCODING,
        header::CONTENT_LENGTH,
    ] {
        headers.remove(name);
    }
    headers.remove("keep-alive");
    let mut response = Response::new(Body::from_stream(reply.bytes_stream()));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}

fn respond(entry: Arc<Entry>) -> Response {
    let recorded = &entry.response;
    let body = match (&recorded.body, &recorded.events) {
        (Some(body), _) => Body::from(body.clone()),
        // One chunk per event, no delays.
        (None, events) => Body::from_stream(futures_util::stream::iter(
            events
                .clone()
                .into_iter()
                .flatten()
                .map(Ok::<_, Infallible>),
        )),
    };
    let mut response = Response::new(body);
    // Cassette statuses are validated to 100-999 on load.
    *response.status_mut() =
        StatusCode::from_u16(recorded.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    if let Some(ctype) = recorded
        .content_type
        .as_deref()
        .and_then(|c| HeaderValue::from_str(c).ok())
    {
        response.headers_mut().insert(header::CONTENT_TYPE, ctype);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sum(body: &str) -> MissRequest {
        summarize("POST", "/v1/chat/completions", body.as_bytes())
    }

    #[test]
    fn miss_summary_handles_chat_responses_and_messages_shapes() {
        let chat = sum(
            r#"{"model":"gpt-5-mini","messages":[{"role":"system","content":"s"},{"role":"user","content":"make this loud: hello"}]}"#,
        );
        assert_eq!(chat.model.as_deref(), Some("gpt-5-mini"));
        assert_eq!(chat.messages, Some(2));
        assert_eq!(chat.last_message.as_deref(), Some("make this loud: hello"));

        let responses = sum(r#"{"input":"just a string"}"#);
        assert_eq!(responses.model, None);
        assert_eq!(responses.messages, Some(1));
        assert_eq!(responses.last_message.as_deref(), Some("just a string"));

        let blocks = sum(
            r#"{"model":"claude","messages":[{"role":"user","content":[{"type":"text","text":"a"},{"type":"image"},{"type":"text","text":"b"}]}]}"#,
        );
        assert_eq!(blocks.last_message.as_deref(), Some("a\nb"));
        let items = sum(
            r#"{"input":[{"role":"user","content":"q"},{"type":"function_call_output","output":"x"}]}"#,
        );
        assert_eq!((items.messages, items.last_message), (Some(2), None));

        let long = "é".repeat(1000);
        let truncated = sum(&format!(
            r#"{{"messages":[{{"role":"user","content":"{long}"}}]}}"#
        ))
        .last_message
        .unwrap();
        assert_eq!(truncated.chars().count(), 121);
        assert!(truncated.ends_with("é…"));
        assert_eq!(sum(r#"{"messages":[]}"#).last_message, None);

        for odd in ["", "[1,2]", r#"{"model":7,"messages":"x","input":null}"#] {
            let s = summarize("GET", "/v1/models", odd.as_bytes());
            assert_eq!(
                (s.model, s.messages, s.last_message),
                (None, None, None),
                "{odd}"
            );
            assert_eq!((s.method.as_str(), s.path.as_str()), ("GET", "/v1/models"));
        }
    }

    #[test]
    fn default_upstream_hint_depends_on_path() {
        assert_eq!(
            default_upstream("/v1/messages"),
            "https://api.anthropic.com"
        );
        assert_eq!(
            default_upstream("/anthropic/v1/messages"),
            "https://api.anthropic.com"
        );
        assert_eq!(
            default_upstream("/v1/chat/completions"),
            "https://api.openai.com"
        );
        assert_eq!(default_upstream("/v1/responses"), "https://api.openai.com");
        assert_eq!(
            default_upstream("/v1/messages/count_tokens"),
            "https://api.openai.com"
        );
    }
}
