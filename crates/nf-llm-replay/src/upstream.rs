//! The HTTP client for record and passthrough: header filtering and API key injection.

use std::time::Duration;

use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, header};

use crate::config::ApiKey;

/// Longest wait for a whole upstream response (LLM answers can be slow).
const TIMEOUT: Duration = Duration::from_secs(600);
/// Longest wait for the TCP/TLS connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Client headers never sent upstream: credentials, hop-by-hop, framing and compression.
const DROPPED: &[&str] = &[
    "authorization",
    "x-api-key",
    "api-key",
    "x-goog-api-key",
    "cookie",
    "connection",
    "upgrade",
    "http2-settings",
    "keep-alive",
    "te",
    "trailer",
    "transfer-encoding",
    "host",
    "content-length",
    "accept-encoding",
];

/// Why the upstream client cannot be built.
#[derive(Debug, thiserror::Error)]
pub enum UpstreamError {
    /// The key contains characters not allowed in an HTTP header.
    #[error("NF_LLM_REPLAY_API_KEY is not a valid HTTP header value")]
    Key,
    /// TLS or client setup failed.
    #[error("cannot build the upstream HTTP client: {0}")]
    Client(#[from] reqwest::Error),
}

/// The provider behind the proxy; it holds the only copy of the real credential.
#[derive(Debug)]
pub struct Upstream {
    client: reqwest::Client,
    /// Base URL without a trailing `/`.
    base: String,
    /// The key as sent in `x-api-key` / `api-key`.
    key: HeaderValue,
    /// `Bearer <key>` for `authorization`.
    bearer: HeaderValue,
}

impl Upstream {
    /// A client for `base` (e.g. `https://api.openai.com`) that injects `key`; never follows redirects.
    pub fn new(base: &str, key: ApiKey) -> Result<Self, UpstreamError> {
        let header = |value: String| {
            let mut value = HeaderValue::try_from(value).map_err(|_| UpstreamError::Key)?;
            value.set_sensitive(true);
            Ok::<_, UpstreamError>(value)
        };
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(TIMEOUT)
                .connect_timeout(CONNECT_TIMEOUT)
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            base: base.trim_end_matches('/').to_owned(),
            bearer: header(format!("Bearer {}", key.expose()))?,
            key: header(key.0)?,
        })
    }

    /// Sends the request to base + `path_and_query` with the original body bytes.
    /// Errors never include the URL (the client's query may hold its own credential).
    pub async fn send(
        &self,
        method: Method,
        path_and_query: &str,
        headers: &HeaderMap,
        body: Bytes,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .request(method, format!("{}{path_and_query}", self.base))
            .headers(self.forward_headers(headers))
            .body(body)
            .send()
            .await
            .map_err(reqwest::Error::without_url)
    }

    /// The client headers that may go upstream plus the key, in the header style the client used.
    pub(crate) fn forward_headers(&self, client: &HeaderMap) -> HeaderMap {
        let mut out: HeaderMap = client
            .iter()
            .filter(|(name, _)| {
                let name = name.as_str();
                !DROPPED.contains(&name) && !name.starts_with("proxy-")
            })
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        let (name, value) = if client.contains_key("x-api-key") {
            (HeaderName::from_static("x-api-key"), &self.key)
        } else if client.contains_key("api-key") {
            (HeaderName::from_static("api-key"), &self.key)
        } else {
            (header::AUTHORIZATION, &self.bearer)
        };
        out.insert(name, value.clone());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue, Method};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const KEY: &str = "sk-test-SECRET-123";

    fn upstream(base: &str) -> Upstream {
        Upstream::new(base, ApiKey(KEY.into())).unwrap()
    }

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), HeaderValue::from_static(v)))
            .collect()
    }

    #[test]
    fn hop_by_hop_credential_and_encoding_headers_are_not_forwarded() {
        let client = headers(&[
            ("authorization", "Bearer nxf-no-credential"),
            ("x-goog-api-key", "g"),
            ("cookie", "c=1"),
            ("connection", "Upgrade, HTTP2-Settings"),
            ("upgrade", "h2c"),
            ("http2-settings", "AAMAAABkAARAAAAAAAIAAAAA"),
            ("keep-alive", "timeout=5"),
            ("te", "trailers"),
            ("trailer", "x"),
            ("transfer-encoding", "chunked"),
            ("proxy-authorization", "Basic x"),
            ("proxy-connection", "keep-alive"),
            ("host", "localhost:8080"),
            ("content-length", "2"),
            ("accept-encoding", "gzip"),
            ("anthropic-version", "2023-06-01"),
            ("anthropic-beta", "tools-2024"),
            ("content-type", "application/json"),
            ("user-agent", "langchain4j"),
        ]);
        let sent = upstream("http://up").forward_headers(&client);
        let names: Vec<&str> = sent.keys().map(|k| k.as_str()).collect();
        assert_eq!(
            names,
            [
                "anthropic-version",
                "anthropic-beta",
                "content-type",
                "user-agent",
                "authorization"
            ],
            "{sent:?}"
        );
        assert_eq!(sent["anthropic-version"], "2023-06-01");
        assert_eq!(sent["authorization"], format!("Bearer {KEY}").as_str());
        assert!(sent["authorization"].is_sensitive());
        assert!(
            upstream("http://up")
                .forward_headers(&HeaderMap::new())
                .len()
                == 1
        );
    }

    #[test]
    fn api_key_is_injected_in_the_client_header_style() {
        let up = upstream("http://up");
        let anthropic = up.forward_headers(&headers(&[("x-api-key", "dummy")]));
        assert_eq!(anthropic.len(), 1);
        assert_eq!(anthropic["x-api-key"], KEY);
        assert!(anthropic["x-api-key"].is_sensitive());

        let azure = up.forward_headers(&headers(&[("api-key", "dummy")]));
        assert_eq!((azure.len(), azure["api-key"].to_str().unwrap()), (1, KEY));

        let both = up.forward_headers(&headers(&[("api-key", "a"), ("x-api-key", "b")]));
        assert_eq!((both.len(), both["x-api-key"].to_str().unwrap()), (1, KEY));

        let bearer = up.forward_headers(&headers(&[("authorization", "Bearer dummy")]));
        assert_eq!(bearer["authorization"], format!("Bearer {KEY}").as_str());

        let bad = Upstream::new("http://up", ApiKey("sk\nnewline".into()));
        assert!(matches!(bad, Err(UpstreamError::Key)));
    }

    #[tokio::test]
    async fn send_targets_base_path_and_query_without_following_redirects() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/prefix/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 4096];
            while !request.ends_with(b"{\"a\":1}") {
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0, "connection closed early");
                request.extend_from_slice(&buf[..n]);
            }
            let reply =
                "HTTP/1.1 302 Found\r\nlocation: http://127.0.0.1:1/\r\ncontent-length: 0\r\n\r\n";
            socket.write_all(reply.as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        });
        let client = headers(&[("x-api-key", "dummy"), ("anthropic-version", "v")]);
        let response = upstream(&base)
            .send(
                Method::POST,
                "/v1/messages?beta=true",
                &client,
                "{\"a\":1}".into(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 302);
        let request = server.await.unwrap().to_ascii_lowercase();
        assert!(
            request.starts_with("post /prefix/v1/messages?beta=true http/1.1\r\n"),
            "{request}"
        );
        assert!(request.contains(&format!("x-api-key: {}\r\n", KEY.to_ascii_lowercase())));
        assert!(request.contains("anthropic-version: v\r\n"));
        assert!(!request.contains("dummy") && !request.contains("accept-encoding"));
    }

    #[tokio::test]
    async fn send_error_omits_the_url() {
        let err = upstream("http://127.0.0.1:1")
            .send(
                Method::GET,
                "/v1/models?key=client-secret",
                &HeaderMap::new(),
                "".into(),
            )
            .await
            .unwrap_err();
        assert!(!err.to_string().contains("client-secret"), "{err}");
    }
}
