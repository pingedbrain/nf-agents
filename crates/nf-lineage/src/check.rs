//! `nextflow lineage check`: verify a FileOutput's recorded checksum against the data file.

use std::io::ErrorKind;
use std::path::{Path as FsPath, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use object_store::ObjectStore;
use object_store::path::Path;
use serde::Serialize;

use crate::{Lid, LineageError, LineageStore, ObjectLineageStore, Record};

/// Options for [`check`].
#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    /// `(recorded prefix, local dir)` pairs: a local recorded path under a prefix (whole path
    /// components) is read below the local dir instead; the first match wins. The checksum still
    /// hashes the recorded path.
    pub path_map: Vec<(String, PathBuf)>,
    /// `object_store` config keys for `s3://` data files (see
    /// [`ObjectLineageStore::open_with_options`]).
    pub storage_options: Vec<(String, String)>,
}

/// The outcome of [`check`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckReport {
    /// The checked FileOutput.
    pub lid: Lid,
    /// The data file path as recorded.
    pub path: String,
    /// The verdict.
    pub status: CheckStatus,
    /// The recorded checksum value.
    pub expected: Option<String>,
    /// The checksum computed now (`Valid`/`Mismatch` only).
    pub actual: Option<String>,
}

/// The verdict of [`check`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// The data file matches the recorded checksum.
    Valid,
    /// The data file changed (content size, mtime or path).
    Mismatch,
    /// The data file does not exist.
    FileMissing,
    /// The record has no checksum (the CLI reports success).
    NoChecksum,
}

/// Checks the FileOutput `lid` like `nextflow lineage check`: the `nextflow`/`standard` checksum
/// is murmur3 over the recorded path, size and mtime. Local paths (plain or `file://`) and
/// `s3://` are supported. Read-only.
///
/// # Errors
/// [`LineageError::UnexpectedKind`] unless `lid` is a FileOutput,
/// [`LineageError::UnsupportedChecksum`] for another algorithm or mode,
/// [`LineageError::Unsupported`] for other schemes (`gs://`, `az://`, ...), store and I/O errors.
pub async fn check(
    store: &dyn LineageStore,
    lid: &Lid,
    opts: &CheckOptions,
) -> Result<CheckReport, LineageError> {
    let file = match store.get(lid).await? {
        Record::FileOutput(file) => file,
        other => {
            return Err(LineageError::UnexpectedKind {
                lid: lid.to_string(),
                expected: "FileOutput",
                found: other.kind().to_owned(),
            });
        }
    };
    let path = file.path.ok_or_else(|| LineageError::InvalidFormat {
        path: lid.to_string(),
        message: "FileOutput without a path".to_owned(),
    })?;
    let mut report = CheckReport {
        lid: lid.clone(),
        path: path.clone(),
        status: CheckStatus::NoChecksum,
        expected: None,
        actual: None,
    };
    let Some(checksum) = file.checksum else {
        return Ok(report);
    };
    let algorithm = checksum.algorithm.unwrap_or_default();
    let mode = checksum.mode.unwrap_or_default();
    if algorithm != "nextflow" || !(mode.is_empty() || mode.eq_ignore_ascii_case("standard")) {
        return Err(LineageError::UnsupportedChecksum {
            path,
            algorithm,
            mode,
        });
    }
    report.expected = checksum.value;
    let Some((hashed, size, mtime_ms)) = stat(&path, opts).await? else {
        report.status = CheckStatus::FileMissing;
        return Ok(report);
    };
    let actual = nextflow_standard_checksum(&hashed, size, mtime_ms);
    report.status = if report.expected.as_ref() == Some(&actual) {
        CheckStatus::Valid
    } else {
        CheckStatus::Mismatch
    };
    report.actual = Some(actual);
    Ok(report)
}

/// Nextflow's `standard` file checksum (Guava `murmur3_128`, seed 0): UTF-16LE of the hashed
/// path, then the size and the mtime in milliseconds as little-endian `i64`; lowercase hex.
pub fn nextflow_standard_checksum(hashed_path: &str, size: u64, mtime_ms: i64) -> String {
    let mut data: Vec<u8> = hashed_path
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    data.extend_from_slice(&(size as i64).to_le_bytes());
    data.extend_from_slice(&mtime_ms.to_le_bytes());
    // Reading from a slice cannot fail.
    let hash = murmur3::murmur3_x64_128(&mut data.as_slice(), 0).unwrap_or_default();
    hash.to_le_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `(hashed path, size, mtime ms)` of the data file, `None` when it does not exist.
async fn stat(path: &str, opts: &CheckOptions) -> Result<Option<(String, u64, i64)>, LineageError> {
    if let Some(rest) = path.strip_prefix("s3://") {
        let Some((bucket, key)) = rest
            .split_once('/')
            .filter(|(b, k)| !b.is_empty() && !k.is_empty())
        else {
            return Err(LineageError::Unsupported(format!(
                "S3 path without a key: {path}"
            )));
        };
        let store = ObjectLineageStore::open_with_options(
            &format!("s3://{bucket}"),
            opts.storage_options.iter().map(|(k, v)| (k, v.clone())),
        )?;
        let stat = object_stat(store.store.as_ref(), key).await?;
        return Ok(stat.map(|(size, ms)| (format!("/{rest}"), size, ms)));
    }
    let hashed = match path.strip_prefix("file://") {
        Some(_) => url::Url::parse(path)
            .ok()
            .and_then(|u| u.to_file_path().ok())
            .map(|p| p.to_string_lossy().into_owned())
            .ok_or_else(|| LineageError::Unsupported(format!("file URL: {path}")))?,
        None if path.contains("://") => {
            return Err(LineageError::Unsupported(format!(
                "checking data files at {path} (only local and s3:// paths)"
            )));
        }
        None => path.to_owned(),
    };
    let local = local_path(&hashed, opts)?;
    match std::fs::metadata(&local) {
        Ok(meta) => {
            let modified = meta.modified().map_err(|source| io(&local, source))?;
            Ok(Some((hashed, meta.len(), floor_ms(modified))))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(io(&local, source)),
    }
}

fn io(path: &FsPath, source: std::io::Error) -> LineageError {
    LineageError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Applies the first matching `path_map` prefix. A recorded path with a `..` component is
/// refused before the map applies, so it cannot climb out of the mapped directory.
fn local_path(recorded: &str, opts: &CheckOptions) -> Result<PathBuf, LineageError> {
    if FsPath::new(recorded)
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(LineageError::Unsupported(format!(
            "recorded path {recorded} has a '..' component"
        )));
    }
    Ok(opts
        .path_map
        .iter()
        .find_map(|(prefix, dir)| {
            FsPath::new(recorded)
                .strip_prefix(prefix)
                .ok()
                .map(|rest| dir.join(rest))
        })
        .unwrap_or_else(|| recorded.into()))
}

/// Size and mtime (ms) of `key` from a listing of its parent prefix: S3 `HEAD` drops the
/// milliseconds Nextflow hashed, the listing keeps them.
async fn object_stat(
    store: &dyn ObjectStore,
    key: &str,
) -> Result<Option<(u64, i64)>, LineageError> {
    let location = Path::parse(key).map_err(object_store::Error::from)?;
    let parent = match key.rsplit_once('/') {
        Some((parent, _)) => Some(Path::parse(parent).map_err(object_store::Error::from)?),
        None => None,
    };
    let listing = store.list_with_delimiter(parent.as_ref()).await?;
    Ok(listing
        .objects
        .into_iter()
        .find(|o| o.location == location)
        .map(|o| (o.size, o.last_modified.timestamp_millis())))
}

/// Milliseconds since the epoch, floored like Java's `FileTime.toMillis`.
fn floor_ms(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(e) => -(e.duration().as_nanos().div_ceil(1_000_000) as i64),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, UNIX_EPOCH};

    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::{ObjectStoreExt, PutPayload};

    use super::*;
    use crate::{LineageError, ObjectLineageStore};

    #[test]
    fn golden_vectors() {
        for (path, size, mtime, want) in [
            (
                "/lineage/results/summary.txt",
                9,
                1_795_726_937_562,
                "c47bf938e54f24686b01873ab8d1ba53",
            ),
            (
                "/nf-agents-fixture/results-hello/summary.txt",
                9,
                1_795_726_427_044,
                "87105ea50d9e8dca5da368e47d34d9bc",
            ),
            ("", 0, 0, "d618a97df21bbd4bb61c79cdeca965b4"),
            (
                "/data/ünïcode 🧬.txt",
                35,
                -1500,
                "9ad8de60441a469eb4b8c0e1bca74c1a",
            ),
        ] {
            assert_eq!(
                nextflow_standard_checksum(path, size, mtime),
                want,
                "{path}"
            );
        }
        // Java `FileTime.toMillis` floors, also before the epoch.
        assert_eq!(
            floor_ms(UNIX_EPOCH - Duration::from_micros(1_499_500)),
            -1500
        );
        assert_eq!(floor_ms(UNIX_EPOCH + Duration::from_micros(1_999)), 1);
    }

    #[tokio::test]
    async fn object_stat_lists_the_parent_prefix() {
        let memory = InMemory::new();
        for key in ["b/results/summary.txt", "b/results/summary.txt2", "top.txt"] {
            let payload = PutPayload::from(b"123456789".to_vec());
            memory
                .put(&Path::parse(key).unwrap(), payload)
                .await
                .unwrap();
        }
        let (size, _) = object_stat(&memory, "b/results/summary.txt")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(size, 9);
        assert!(object_stat(&memory, "top.txt").await.unwrap().is_some());
        assert!(
            object_stat(&memory, "b/results/none.txt")
                .await
                .unwrap()
                .is_none()
        );
        assert!(object_stat(&memory, "b/results").await.unwrap().is_none());
    }

    const H: &str = "6d6d78525bd5066888a8696fbe711344";

    /// A store holding one FileOutput (`lid://H/f`) with `path` and `checksum` JSON, and a
    /// TaskRun (`lid://H`).
    async fn store(path: &str, checksum: &str) -> ObjectLineageStore {
        let memory = InMemory::new();
        let file = format!(
            r#"{{"version":"lineage/v1beta1","kind":"FileOutput","spec":{{"path":"{path}","checksum":{checksum}}}}}"#
        );
        let task = r#"{"version":"lineage/v1beta1","kind":"TaskRun","spec":{}}"#.to_owned();
        for (key, body) in [
            (format!("s/{H}/f/.data.json"), file),
            (format!("s/{H}/.data.json"), task),
        ] {
            let payload = PutPayload::from(body.into_bytes());
            memory
                .put(&Path::parse(key).unwrap(), payload)
                .await
                .unwrap();
        }
        ObjectLineageStore::new(Arc::new(memory), Path::parse("s").unwrap())
    }

    async fn run(path: &str, checksum: &str) -> Result<CheckReport, LineageError> {
        let file = Lid::parse(&format!("lid://{H}/f")).unwrap();
        check(
            &store(path, checksum).await,
            &file,
            &CheckOptions::default(),
        )
        .await
    }

    #[tokio::test]
    async fn rejects_what_it_cannot_verify() {
        let ok = r#"{"value":"x","algorithm":"nextflow","mode":"STANDARD"}"#;
        let report = run("/no/such/file", "null").await.unwrap();
        assert_eq!(report.status, CheckStatus::NoChecksum);
        let report = run("/no/such/file", ok).await.unwrap();
        assert_eq!(report.status, CheckStatus::FileMissing);
        for checksum in [
            r#"{"value":"x","algorithm":"sha256"}"#,
            r#"{"value":"x","mode":"standard"}"#,
            r#"{"value":"x","algorithm":"nextflow","mode":"deep"}"#,
        ] {
            let err = run("/no/such/file", checksum).await.unwrap_err();
            assert!(
                matches!(err, LineageError::UnsupportedChecksum { .. }),
                "{checksum}"
            );
        }
        for path in ["gs://b/k", "az://c/k", "http://h/k", "s3://bucket-only"] {
            let err = run(path, ok).await.unwrap_err();
            assert!(matches!(err, LineageError::Unsupported(_)), "{path}: {err}");
        }
        let task = Lid::parse(&format!("lid://{H}")).unwrap();
        let err = check(&store("/x", ok).await, &task, &CheckOptions::default()).await;
        assert!(matches!(err, Err(LineageError::UnexpectedKind { .. })));
    }

    #[test]
    fn rejects_parent_components_before_path_map() {
        let opts = CheckOptions {
            path_map: vec![("/rec".to_owned(), "/local".into())],
            ..CheckOptions::default()
        };
        for recorded in ["/rec/../secret", "/other/../x", "/rec/a/../../etc"] {
            assert!(
                matches!(
                    local_path(recorded, &opts),
                    Err(LineageError::Unsupported(_))
                ),
                "{recorded}"
            );
        }
        assert!(local_path("/rec/a..b/c", &opts).is_ok());
    }

    #[tokio::test]
    async fn check_refuses_parent_components_in_recorded_paths() {
        let path = "/no/such/dir/../file";
        let ok = r#"{"value":"x","algorithm":"nextflow","mode":"STANDARD"}"#;
        let err = run(path, ok).await.unwrap_err();
        assert!(matches!(err, LineageError::Unsupported(_)), "{err}");
    }

    #[test]
    fn path_map_remaps_on_component_boundaries() {
        let opts = CheckOptions {
            path_map: vec![("/rec".to_owned(), "/local".into())],
            ..CheckOptions::default()
        };
        assert_eq!(
            local_path("/rec/a/b.txt", &opts).unwrap(),
            std::path::Path::new("/local/a/b.txt")
        );
        assert_eq!(
            local_path("/record/b.txt", &opts).unwrap(),
            std::path::Path::new("/record/b.txt")
        );
    }
}
