//! Shared integration-test helpers: in-process server, cassette writer, raw HTTP/1.1 client.

// Each test binary uses a subset of these; helpers outside #[test] fns may unwrap.
#![allow(dead_code, clippy::unwrap_used, clippy::indexing_slicing)]

pub mod fake_upstream;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::Parser;
use nf_llm_replay::cassette::{Entry, RecordedRequest, RecordedResponse};
use nf_llm_replay::config::{Cli, Config};
use nf_llm_replay::normalize::Normalizer;
use serde_json::value::RawValue;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Serves `app(config)` on an ephemeral local port.
pub async fn start(config: Config) -> SocketAddr {
    let app = nf_llm_replay::app(&config).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

/// Config parsed from command line flags.
pub fn config(args: &[&str]) -> Config {
    let cli = Cli::try_parse_from(std::iter::once("nf-llm-replay").chain(args.iter().copied()));
    Config::from_cli(cli.unwrap(), None).unwrap()
}

/// The upstream key the record/passthrough helpers set.
pub const KEY: &str = "sk-test-SECRET-123";

/// Starts `--mode <mode>` for `cassette` against `upstream`, with [`KEY`] as the env key.
pub async fn proxy(mode: &str, cassette: &Path, upstream: &str) -> SocketAddr {
    let args = [
        "--mode",
        mode,
        "--cassette",
        cassette.to_str().unwrap(),
        "--upstream",
        upstream,
    ];
    let cli = Cli::try_parse_from(std::iter::once("nf-llm-replay").chain(args)).unwrap();
    start(Config::from_cli(cli, Some(KEY.into())).unwrap()).await
}

/// Starts a replay server for `cassette`.
pub async fn replay(cassette: &Path) -> SocketAddr {
    start(config(&["--cassette", cassette.to_str().unwrap()])).await
}

/// One cassette line for a request/response pair, keyed like the proxy does.
pub fn entry(
    method: &str,
    path: &str,
    body: &str,
    status: u16,
    ctype: Option<&str>,
    resp: &str,
) -> String {
    let norm = Normalizer::new(&[], &[])
        .unwrap()
        .normalize(method, path, body.as_bytes())
        .unwrap();
    let entry = Entry {
        v: 1,
        key: norm.key,
        request: RecordedRequest {
            method: method.into(),
            path: path.into(),
            body: (!norm.canonical.is_empty())
                .then(|| RawValue::from_string(norm.canonical).unwrap()),
        },
        response: RecordedResponse {
            status,
            content_type: ctype.map(Into::into),
            body: Some(resp.into()),
            events: None,
        },
    };
    serde_json::to_string(&entry).unwrap()
}

/// Writes `lines` as `dir/cassette.jsonl`.
pub fn write_cassette(dir: &Path, lines: &[String]) -> PathBuf {
    let path = dir.join("cassette.jsonl");
    std::fs::write(
        &path,
        lines.iter().map(|l| format!("{l}\n")).collect::<String>(),
    )
    .unwrap();
    path
}

/// A parsed HTTP/1.1 response.
#[derive(Debug, PartialEq, Eq)]
pub struct Reply {
    /// Status line, e.g. `HTTP/1.1 200 OK`.
    pub status_line: String,
    /// Status code.
    pub status: u16,
    /// Header lines, names lowercased.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap()
    }
    pub fn text(&self) -> String {
        String::from_utf8(self.body.clone()).unwrap()
    }
}

/// Sends raw request bytes and reads one response (content-length or chunked framing, else
/// until EOF).
pub async fn send_raw(addr: SocketAddr, request: &[u8]) -> Reply {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request).await.unwrap();
    let mut buf = Vec::new();
    let end = loop {
        if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break end;
        }
        read_more(&mut stream, &mut buf).await;
    };
    let head = String::from_utf8(buf[..end].to_vec()).unwrap();
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap().to_owned();
    let status = status_line.split(' ').nth(1).unwrap().parse().unwrap();
    let headers: Vec<(String, String)> = lines
        .map(|l| {
            let (n, v) = l.split_once(':').unwrap();
            (n.to_ascii_lowercase(), v.trim().to_owned())
        })
        .collect();
    let mut body = buf.split_off(end + 4);
    let len = headers.iter().find(|(n, _)| n == "content-length");
    match len.map(|(_, v)| v.parse::<usize>().unwrap()) {
        Some(len) => {
            while body.len() < len {
                read_more(&mut stream, &mut body).await;
            }
            body.truncate(len);
        }
        None if headers.contains(&("transfer-encoding".into(), "chunked".into())) => loop {
            if let Some(decoded) = dechunk(&body) {
                body = decoded;
                break;
            }
            read_more(&mut stream, &mut body).await;
        },
        None => {
            stream.read_to_end(&mut body).await.unwrap();
        }
    }
    Reply {
        status_line,
        status,
        headers,
        body,
    }
}

/// The decoded body of a complete chunked message (trailers ignored); `None` while incomplete.
fn dechunk(mut raw: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let eol = raw.windows(2).position(|w| w == b"\r\n")?;
        let size = std::str::from_utf8(&raw[..eol]).ok()?.split(';').next()?;
        let size = usize::from_str_radix(size.trim(), 16).ok()?;
        if size == 0 {
            return Some(out);
        }
        out.extend_from_slice(raw.get(eol + 2..eol + 2 + size)?);
        raw = raw.get(eol + 4 + size..)?;
    }
}

async fn read_more(stream: &mut TcpStream, buf: &mut Vec<u8>) {
    let mut chunk = [0u8; 65536];
    let n = stream.read(&mut chunk).await.unwrap();
    assert!(n > 0, "connection closed before a full response");
    buf.extend_from_slice(&chunk[..n]);
}

/// `POST path` with a JSON body.
pub async fn post(addr: SocketAddr, path: &str, body: &[u8]) -> Reply {
    post_with(addr, path, "", body).await
}

/// `POST path` with extra header lines (each ending in `\r\n`) and a JSON body.
pub async fn post_with(addr: SocketAddr, path: &str, headers: &str, body: &[u8]) -> Reply {
    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: test\r\n{headers}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    req.extend_from_slice(body);
    send_raw(addr, &req).await
}

/// A listener that only counts connections: a live "upstream" replay must never touch.
pub async fn counting_upstream() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    tokio::spawn(async move {
        while listener.accept().await.is_ok() {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    });
    (url, hits)
}
