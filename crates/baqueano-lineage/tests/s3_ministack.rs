//! S3 integration tests against MiniStack (`ministackorg/ministack:1.5.45`).
//!
//! Gated on `BAQUEANO_LINEAGE_MINISTACK_URL` (e.g. `http://localhost:4566`): unset → skip notice and
//! pass; set → wait up to 60 s for MiniStack, then fail on any error. The tests write their own
//! buckets with dummy `test`/`test` credentials; the crate under test only reads.
// Integration tests: panicking on unexpected state is the failure mode we want.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

use baqueano_lineage::object_store::aws::AmazonS3Builder;
use baqueano_lineage::object_store::local::LocalFileSystem;
use baqueano_lineage::object_store::memory::InMemory;
use baqueano_lineage::object_store::path::Path;
use baqueano_lineage::object_store::{ObjectStore, ObjectStoreExt, PutPayload};
use baqueano_lineage::{
    CheckOptions, CheckStatus, FindQuery, Lid, LineageStore, ObjectLineageStore, Record, check,
    nextflow_standard_checksum,
};
use futures::TryStreamExt;

const ENV: &str = "BAQUEANO_LINEAGE_MINISTACK_URL";
const READY_TIMEOUT: Duration = Duration::from_secs(60);

/// The MiniStack endpoint, or `None` (with a skip notice) when the variable is unset.
fn endpoint(test: &str) -> Option<String> {
    let url = std::env::var(ENV).ok().filter(|v| !v.is_empty());
    if url.is_none() {
        eprintln!("skipping {test}: {ENV} is not set (see CONTRIBUTING.md)");
    }
    url
}

/// `object_store` config keys for MiniStack, independent of the caller's AWS environment.
fn options(endpoint: &str) -> Vec<(String, String)> {
    [
        ("aws_endpoint", endpoint),
        ("aws_allow_http", "true"),
        ("aws_region", "us-east-1"),
        ("aws_access_key_id", "test"),
        ("aws_secret_access_key", "test"),
    ]
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .to_vec()
}

/// Creates `bucket` with a raw `PUT /<bucket>` (object_store cannot create buckets), retrying
/// until MiniStack answers 2xx or [`READY_TIMEOUT`] passes. Doubles as the readiness wait.
fn create_bucket(endpoint: &str, bucket: &str) {
    let url = url::Url::parse(endpoint).unwrap();
    let host = format!(
        "{}:{}",
        url.host_str().unwrap(),
        url.port_or_known_default().unwrap()
    );
    let request = format!(
        "PUT /{bucket} HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let deadline = Instant::now() + READY_TIMEOUT;
    let mut last = String::new();
    while Instant::now() < deadline {
        let attempt = TcpStream::connect(&host).and_then(|mut tcp| {
            tcp.set_read_timeout(Some(Duration::from_secs(5)))?;
            tcp.write_all(request.as_bytes())?;
            let mut response = String::new();
            tcp.read_to_string(&mut response)?;
            Ok(response)
        });
        match attempt {
            Ok(response) if response.starts_with("HTTP/1.1 2") => return,
            Ok(response) => last = response.lines().next().unwrap_or_default().to_owned(),
            Err(error) => last = error.to_string(),
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("MiniStack at {endpoint} not ready after {READY_TIMEOUT:?}: {last}");
}

fn s3(endpoint: &str, bucket: &str) -> Arc<dyn ObjectStore> {
    let mut builder = AmazonS3Builder::new().with_bucket_name(bucket);
    for (key, value) in options(endpoint) {
        builder = builder.with_config(key.parse().unwrap(), value);
    }
    Arc::new(builder.build().unwrap())
}

#[tokio::test]
async fn s3_store_roundtrip() {
    let Some(endpoint) = endpoint("s3_store_roundtrip") else {
        return;
    };
    let bucket = "baqueano-lineage-roundtrip";
    create_bucket(&endpoint, bucket);

    // Upload the 26.10.7 store below `store/`, keeping listed keys (`<hash>#output/…`) verbatim.
    let dir = common::store_dir("26.10.7");
    let local = LocalFileSystem::new_with_prefix(&dir).unwrap();
    let remote = s3(&endpoint, bucket);
    let files: Vec<_> = local.list(None).try_collect().await.unwrap();
    assert!(
        files
            .iter()
            .any(|f| f.location.as_ref().contains("#output"))
    );
    for file in &files {
        let data = local
            .get(&file.location)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        let key = Path::parse(format!("store/{}", file.location)).unwrap();
        remote.put(&key, PutPayload::from(data)).await.unwrap();
    }

    let expected = ObjectLineageStore::open(dir.to_str().unwrap()).unwrap();
    let store: Box<dyn LineageStore> = Box::new(
        ObjectLineageStore::open_with_options(&format!("s3://{bucket}/store"), options(&endpoint))
            .unwrap(),
    );

    let runs = store.list_runs().await.unwrap();
    assert!(!runs.is_empty());
    assert_eq!(runs, expected.list_runs().await.unwrap());

    let everything = FindQuery::parse_args::<&str>([]).unwrap();
    let all = store.find(&everything).await.unwrap();
    assert_eq!(all, expected.find(&everything).await.unwrap());
    let outputs: Vec<&Lid> = all
        .iter()
        .filter(|l| l.fragment() == Some("output"))
        .collect();
    assert!(!outputs.is_empty(), "no #output record in the fixture");
    for lid in outputs {
        let record = store.get(lid).await.unwrap();
        assert!(matches!(
            record,
            Record::TaskOutput(_) | Record::WorkflowOutput(_)
        ));
        assert_eq!(record, expected.get(lid).await.unwrap());
    }

    let query = FindQuery::parse_args(["type=FileOutput"]).unwrap();
    let files = store.find(&query).await.unwrap();
    assert!(!files.is_empty());
    assert_eq!(files, expected.find(&query).await.unwrap());
}

#[tokio::test]
async fn s3_check_valid_and_mismatch() {
    let Some(endpoint) = endpoint("s3_check_valid_and_mismatch") else {
        return;
    };
    let bucket = "baqueano-lineage-check";
    create_bucket(&endpoint, bucket);
    let remote = s3(&endpoint, bucket);
    let key = Path::parse("results/summary.txt").unwrap();
    remote
        .put(&key, PutPayload::from_static(b"9 bytes\n\n"))
        .await
        .unwrap();

    // Uploading sets the mtime, so build the FileOutput from the listing, as Nextflow does.
    let meta = remote.head(&key).await.unwrap();
    let listed = remote
        .list_with_delimiter(Some(&Path::parse("results").unwrap()))
        .await
        .unwrap()
        .objects
        .into_iter()
        .find(|o| o.location == key)
        .unwrap();
    assert_eq!(listed.size, meta.size);
    let value = nextflow_standard_checksum(
        &format!("/{bucket}/{key}"),
        listed.size,
        listed.last_modified.timestamp_millis(),
    );
    let record = serde_json::json!({
        "version": "lineage/v1beta1",
        "kind": "FileOutput",
        "spec": {
            "path": format!("s3://{bucket}/{key}"),
            "checksum": {"value": value, "algorithm": "nextflow", "mode": "standard"},
            "size": listed.size,
        }
    });
    let records = InMemory::new();
    let hash = "0123456789abcdef0123456789abcdef";
    records
        .put(
            &Path::parse(format!("{hash}/.data.json")).unwrap(),
            PutPayload::from(record.to_string()),
        )
        .await
        .unwrap();
    let store = ObjectLineageStore::new(Arc::new(records), Path::default());
    let lid: Lid = format!("lid://{hash}").parse().unwrap();
    let opts = CheckOptions {
        storage_options: options(&endpoint),
        ..CheckOptions::default()
    };

    let report = check(&store, &lid, &opts).await.unwrap();
    assert_eq!(report.status, CheckStatus::Valid, "{report:?}");
    assert_eq!(report.actual.as_deref(), Some(value.as_str()));

    // A different size changes the checksum even within the same millisecond.
    remote
        .put(&key, PutPayload::from_static(b"rewritten\n"))
        .await
        .unwrap();
    let report = check(&store, &lid, &opts).await.unwrap();
    assert_eq!(report.status, CheckStatus::Mismatch, "{report:?}");
    assert_eq!(report.expected.as_deref(), Some(value.as_str()));
}
