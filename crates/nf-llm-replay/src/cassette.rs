//! JSONL cassette: one recorded request/response pair per line.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, OnceCell};

use crate::config::ApiKey;

/// The only cassette format version this build reads and writes.
pub(crate) const VERSION: u32 = 1;

/// One cassette line. Field order is the on-disk order; unknown fields are ignored.
#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    /// Format version, always `1`.
    pub v: u32,
    /// Request key (see [`crate::normalize`]).
    pub key: String,
    /// The normalized request.
    pub request: RecordedRequest,
    /// The recorded upstream response.
    pub response: RecordedResponse,
}

/// The request part of an entry; never holds headers or the query string.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordedRequest {
    /// HTTP method.
    pub method: String,
    /// URI path as received.
    pub path: String,
    /// Normalized canonical body embedded as JSON; `null` when the body was empty.
    pub body: Option<Box<RawValue>>,
}

/// The response part of an entry: exactly one of `body` or `events` is set.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordedResponse {
    /// HTTP status code.
    pub status: u16,
    /// Upstream `content-type`, if any.
    pub content_type: Option<String>,
    /// Exact response body (non-streamed responses).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Raw server-sent events, each including its terminator (streamed responses).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,
}

/// Why a cassette cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum CassetteError {
    /// The file cannot be read.
    #[error("cannot read cassette `{}`: {source}", path.display())]
    Io {
        /// Cassette path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// A line is not a valid entry.
    #[error("cassette line {line}: {reason}")]
    Line {
        /// 1-based line number.
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// Two lines share a key.
    #[error("cassette key `{key}` appears on lines {first} and {second}")]
    Duplicate {
        /// The repeated key.
        key: String,
        /// 1-based line of the first occurrence.
        first: usize,
        /// 1-based line of the second occurrence.
        second: usize,
    },
    /// The cassette cannot be rewritten.
    #[error("cannot write cassette `{}`: {source}", path.display())]
    Write {
        /// Cassette path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// The entry would store the upstream key (e.g. the provider echoed it).
    #[error("refusing to record an entry that contains the NF_LLM_REPLAY_API_KEY value")]
    Secret,
}

/// An immutable, validated cassette (replay mode).
#[derive(Debug)]
pub struct Cassette {
    entries: BTreeMap<String, Arc<Entry>>,
}

impl Cassette {
    /// Reads and validates a cassette; blank lines are skipped.
    pub fn load(path: &Path) -> Result<Self, CassetteError> {
        let text = std::fs::read_to_string(path).map_err(|source| CassetteError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut entries = BTreeMap::new();
        let mut lines = HashMap::new();
        for (i, text) in text.lines().enumerate() {
            let line = i + 1;
            if text.trim().is_empty() {
                continue;
            }
            let entry = parse_line(text).map_err(|reason| CassetteError::Line { line, reason })?;
            if let Some(&first) = lines.get(&entry.key) {
                return Err(CassetteError::Duplicate {
                    key: entry.key,
                    first,
                    second: line,
                });
            }
            lines.insert(entry.key.clone(), line);
            entries.insert(entry.key.clone(), Arc::new(entry));
        }
        Ok(Self { entries })
    }

    /// The entry recorded for `key`, if any.
    pub fn get(&self, key: &str) -> Option<Arc<Entry>> {
        self.entries.get(key).cloned()
    }
}

/// One recording slot per key: concurrent requests for the same key share one upstream call.
pub type Cell = Arc<OnceCell<Arc<Entry>>>;

/// A cassette being recorded: known keys answer from the file, new entries are persisted sorted.
#[derive(Debug)]
pub struct Recorder {
    path: PathBuf,
    secret: ApiKey,
    saved: Mutex<BTreeMap<String, Arc<Entry>>>,
    cells: std::sync::Mutex<HashMap<String, Cell>>,
}

impl Recorder {
    /// Loads `path` (creating an empty file if absent, so an unwritable location fails at startup).
    pub fn open(path: PathBuf, secret: ApiKey) -> Result<Self, CassetteError> {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| CassetteError::Write {
                path: path.clone(),
                source,
            })?;
        let entries = Cassette::load(&path)?.entries;
        let cells = entries
            .iter()
            .map(|(key, entry)| {
                (
                    key.clone(),
                    Arc::new(OnceCell::new_with(Some(entry.clone()))),
                )
            })
            .collect();
        Ok(Self {
            path,
            secret,
            saved: Mutex::new(entries),
            cells: std::sync::Mutex::new(cells),
        })
    }

    /// The slot for `key`; already filled when the cassette has it.
    pub fn cell(&self, key: &str) -> Cell {
        // The map stays consistent even if a holder panicked, so a poisoned lock is still usable.
        let mut cells = self.cells.lock().unwrap_or_else(PoisonError::into_inner);
        cells.entry(key.to_owned()).or_default().clone()
    }

    /// Adds `entry` and rewrites the whole cassette sorted by key (temp sibling + rename).
    /// Refuses entries containing the upstream key; on error nothing is kept.
    pub async fn persist(&self, entry: Arc<Entry>) -> Result<(), CassetteError> {
        let line = serde_json::to_string(&*entry).map_err(|e| self.write_error(e.into()))?;
        if line.contains(self.secret.expose()) {
            return Err(CassetteError::Secret);
        }
        let key = entry.key.clone();
        let mut saved = self.saved.lock().await;
        saved.insert(key.clone(), entry);
        // ponytail: whole-file rewrite per entry (O(n²) per session); fine for KB-MB cassettes.
        if let Err(e) = write_sorted(&self.path, &saved).await {
            saved.remove(&key);
            return Err(self.write_error(e));
        }
        Ok(())
    }

    fn write_error(&self, source: std::io::Error) -> CassetteError {
        CassetteError::Write {
            path: self.path.clone(),
            source,
        }
    }
}

async fn write_sorted(path: &Path, entries: &BTreeMap<String, Arc<Entry>>) -> std::io::Result<()> {
    let mut text = String::new();
    for entry in entries.values() {
        text += &serde_json::to_string(&**entry)?;
        text.push('\n');
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let mut file = tokio::fs::File::create(&tmp).await?;
    file.write_all(text.as_bytes()).await?;
    file.sync_all().await?;
    tokio::fs::rename(&tmp, path).await
}

fn parse_line(text: &str) -> Result<Entry, String> {
    let entry: Entry = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if entry.v != VERSION {
        return Err(format!(
            "unsupported cassette version {} (expected {VERSION})",
            entry.v
        ));
    }
    let response = &entry.response;
    if !(100..=999).contains(&response.status) {
        return Err(format!("invalid status {}", response.status));
    }
    if response.body.is_some() == response.events.is_some() {
        return Err("response must have exactly one of `body` or `events`".into());
    }
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiKey;
    use serde_json::value::RawValue;

    const JSON: &str = r#"{"v":1,"key":"aa","request":{"method":"POST","path":"/v1/chat/completions","body":{"model":"m"}},"response":{"status":200,"content_type":"application/json","body":"{\"id\":\"x\"}\n"}}"#;
    const SSE: &str = r#"{"v":1,"key":"bb","request":{"method":"POST","path":"/v1/responses","body":null},"response":{"status":200,"content_type":null,"events":["data: 1\n\n","data: 2\r\n\r\n"]}}"#;

    fn load_str(text: &str) -> Result<Cassette, CassetteError> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.jsonl");
        std::fs::write(&path, text).unwrap();
        Cassette::load(&path)
    }

    fn line_error(text: &str) -> (usize, String) {
        match load_str(text) {
            Err(e @ CassetteError::Line { line, .. }) => (line, e.to_string()),
            other => panic!("expected a line error, got {other:?}"),
        }
    }

    #[test]
    fn entry_round_trips_as_one_json_line() {
        let entry = Entry {
            v: 1,
            key: "aa".into(),
            request: RecordedRequest {
                method: "POST".into(),
                path: "/v1/chat/completions".into(),
                body: Some(RawValue::from_string(r#"{"model":"m"}"#.into()).unwrap()),
            },
            response: RecordedResponse {
                status: 200,
                content_type: Some("application/json".into()),
                body: Some("{\"id\":\"x\"}\n".into()),
                events: None,
            },
        };
        let line = serde_json::to_string(&entry).unwrap();
        assert_eq!(line, JSON);
        assert!(!line.contains('\n'));
        for text in [JSON, SSE] {
            let parsed: Entry = serde_json::from_str(text).unwrap();
            assert_eq!(serde_json::to_string(&parsed).unwrap(), text);
        }
        let sse: Entry = serde_json::from_str(SSE).unwrap();
        assert!(sse.request.body.is_none());
        assert_eq!(
            sse.response.events.unwrap().concat(),
            "data: 1\n\ndata: 2\r\n\r\n"
        );
    }

    #[test]
    fn unknown_version_is_rejected_with_line_number() {
        let v2 = SSE.replace(r#"{"v":1"#, r#"{"v":2"#);
        let (line, msg) = line_error(&format!("{JSON}\n\n{v2}\n"));
        assert_eq!(line, 3);
        assert!(msg.contains("line 3") && msg.contains("version 2"), "{msg}");
    }

    #[test]
    fn malformed_line_is_rejected_with_line_number() {
        let (line, msg) = line_error(&format!("{JSON}\n{}\n", &SSE[..40]));
        assert_eq!(line, 2);
        assert!(msg.contains("line 2"), "{msg}");

        let both = JSON.replace(r#""body":"{"#, r#""events":[],"body":"{"#);
        let neither = SSE.replace(r#","events":["data: 1\n\n","data: 2\r\n\r\n"]"#, "");
        for bad in [both, neither] {
            let (line, msg) = line_error(&bad);
            assert_eq!(line, 1);
            assert!(msg.contains("exactly one of `body` or `events`"), "{msg}");
        }
        for status in ["99", "1000"] {
            let (line, msg) = line_error(&JSON.replace("200", status));
            assert_eq!(line, 1);
            assert!(msg.contains("status"), "{msg}");
        }

        let missing = Cassette::load(Path::new("/nonexistent/c.jsonl")).unwrap_err();
        assert!(matches!(missing, CassetteError::Io { .. }));
        assert!(missing.to_string().contains("/nonexistent/c.jsonl"));
    }

    #[test]
    fn duplicate_key_is_rejected_with_both_line_numbers() {
        let err = load_str(&format!("{JSON}\n{SSE}\n{JSON}\n")).unwrap_err();
        assert!(
            matches!(&err, CassetteError::Duplicate { key, first: 1, second: 3 } if key == "aa"),
            "{err:?}"
        );
        assert!(err.to_string().contains("lines 1 and 3"));
    }

    #[test]
    fn unknown_entry_fields_are_ignored() {
        let extra = JSON
            .replace(r#"{"v":1,"#, r#"{"v":1,"note":"later","#)
            .replace(r#""status":200,"#, r#""status":200,"latency_ms":5,"#);
        let cassette = load_str(&format!("{extra}\n{SSE}")).unwrap();
        let entry = cassette.get("aa").unwrap();
        assert_eq!(entry.response.status, 200);
        assert_eq!(entry.response.body.as_deref(), Some("{\"id\":\"x\"}\n"));
        assert_eq!(cassette.get("bb").unwrap().request.path, "/v1/responses");
        assert!(cassette.get("cc").is_none());
    }

    const KEY: &str = "sk-test-SECRET-123";

    fn entry(key: &str, body: &str) -> Arc<Entry> {
        Arc::new(Entry {
            v: 1,
            key: key.into(),
            request: RecordedRequest {
                method: "POST".into(),
                path: "/v1/chat/completions".into(),
                body: None,
            },
            response: RecordedResponse {
                status: 200,
                content_type: Some("application/json".into()),
                body: Some(body.into()),
                events: None,
            },
        })
    }

    fn recorder(dir: &Path) -> Recorder {
        Recorder::open(dir.join("c.jsonl"), ApiKey(KEY.into())).unwrap()
    }

    #[tokio::test]
    async fn insert_rewrites_file_sorted_by_key_without_temp_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.jsonl");
        std::fs::write(&path, format!("{SSE}\n")).unwrap();
        let rec = recorder(dir.path());
        for key in ["cc", "aa", "dd"] {
            rec.persist(entry(key, key)).await.unwrap();
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let keys: Vec<String> = text
            .lines()
            .map(|l| serde_json::from_str::<Entry>(l).unwrap().key)
            .collect();
        assert_eq!(keys, ["aa", "bb", "cc", "dd"]);
        assert!(text.ends_with('\n') && text.contains(SSE));
        let files: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(files.len(), 1, "{files:?}");
        assert_eq!(Cassette::load(&path).unwrap().get("cc").unwrap().key, "cc");

        // A failed write keeps nothing and says where.
        let root = dir.path().to_path_buf();
        drop(dir);
        let err = rec.persist(entry("ee", "x")).await.unwrap_err();
        assert!(matches!(err, CassetteError::Write { .. }), "{err:?}");
        assert!(err.to_string().contains("c.jsonl"), "{err}");
        std::fs::create_dir(&root).unwrap();
        rec.persist(entry("ff", "x")).await.unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(text.lines().count(), 5);
        assert!(!text.contains(r#""key":"ee""#), "{text}");
    }

    #[tokio::test]
    async fn entry_containing_the_api_key_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let rec = recorder(dir.path());
        let path = dir.path().join("c.jsonl");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");

        let echoed = entry("aa", &format!("{{\"echo\":\"Bearer {KEY}\"}}"));
        let err = rec.persist(echoed).await.unwrap_err();
        assert!(matches!(err, CassetteError::Secret), "{err:?}");
        assert!(!err.to_string().contains(KEY), "{err}");

        let mut in_request = entry("bb", "ok");
        Arc::get_mut(&mut in_request).unwrap().request.body =
            Some(RawValue::from_string(format!("{{\"k\":\"{KEY}\"}}")).unwrap());
        assert!(matches!(
            rec.persist(in_request).await,
            Err(CassetteError::Secret)
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");

        rec.persist(entry("cc", "sk-test-other")).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 1);
    }

    #[tokio::test]
    async fn cells_are_shared_per_key_and_prefilled_from_the_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("c.jsonl"), JSON).unwrap();
        let rec = recorder(dir.path());
        assert_eq!(rec.cell("aa").get().unwrap().response.status, 200);
        let new = rec.cell("zz");
        assert!(new.get().is_none());
        assert!(Arc::ptr_eq(&new, &rec.cell("zz")));
        new.get_or_init(|| async { entry("zz", "x") }).await;
        assert_eq!(rec.cell("zz").get().unwrap().key, "zz");

        let corrupt = dir.path().join("bad.jsonl");
        std::fs::write(&corrupt, "{").unwrap();
        let err = Recorder::open(corrupt, ApiKey(KEY.into())).unwrap_err();
        assert!(
            matches!(err, CassetteError::Line { line: 1, .. }),
            "{err:?}"
        );
    }
}
