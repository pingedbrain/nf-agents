//! `LineageStore` over `object_store`: local fixtures, InMemory edge cases, typed errors.
// Integration tests: panicking on unexpected state is the failure mode we want.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::fs;
use std::sync::Arc;

use baqueano_lineage::object_store::memory::InMemory;
use baqueano_lineage::object_store::path::Path;
use baqueano_lineage::object_store::{ObjectStoreExt, PutPayload};
use baqueano_lineage::{FindQuery, Lid, LineageError, LineageStore, ObjectLineageStore, Record};

fn query(args: &[&str]) -> FindQuery {
    FindQuery::parse_args(args).unwrap()
}

#[tokio::test]
async fn local_store_lists_gets_and_finds() {
    for version in common::versions() {
        let dir = common::store_dir(&version);
        let store: Box<dyn LineageStore> =
            Box::new(ObjectLineageStore::open(dir.to_str().unwrap()).unwrap());

        let runs = store.list_runs().await.unwrap();
        assert_eq!(
            runs.len(),
            fs::read_dir(dir.join(".history")).unwrap().count(),
            "{version}"
        );
        let mut sorted = runs.clone();
        sorted.sort_by(|a, b| (&a.timestamp, &a.lid).cmp(&(&b.timestamp, &b.lid)));
        assert_eq!(runs, sorted, "{version}");
        for run in &runs {
            assert!(!run.run_name.is_empty() && !run.session_id.is_empty());
            assert!(matches!(
                store.get(&run.lid).await.unwrap(),
                Record::WorkflowRun(_)
            ));
        }

        let mut run_lids: Vec<Lid> = runs.iter().map(|r| r.lid.clone()).collect();
        run_lids.sort();
        assert_eq!(
            store.find(&query(&["type=WorkflowRun"])).await.unwrap(),
            run_lids,
            "{version}"
        );

        // Every record is found by the empty query, sorted and once.
        let all = store.find(&query(&[])).await.unwrap();
        assert_eq!(all.len(), common::record_files(&dir).len(), "{version}");
        assert!(all.windows(2).all(|w| w[0] < w[1]));

        let task_outputs = store.find(&query(&["type=TaskOutput"])).await.unwrap();
        let lid = task_outputs.first().unwrap();
        assert_eq!(lid.fragment(), Some("output"));
        assert!(
            matches!(store.get(lid).await.unwrap(), Record::TaskOutput(_)),
            "{version}"
        );
        // A fragment other than `output` reads the record itself.
        let run_params = Lid::parse(&format!("{}#params", runs[0].lid)).unwrap();
        assert!(matches!(
            store.get(&run_params).await.unwrap(),
            Record::WorkflowRun(_)
        ));
    }
    let url = format!(
        "file://{}",
        common::store_dir("26.10.7")
            .canonicalize()
            .unwrap()
            .display()
    );
    assert_eq!(
        ObjectLineageStore::open(&url)
            .unwrap()
            .list_runs()
            .await
            .unwrap()
            .len(),
        2
    );
}

const H: &str = "6d6d78525bd5066888a8696fbe711344";

async fn memory_store(entries: &[(&str, &str)]) -> ObjectLineageStore {
    let memory = InMemory::new();
    for (key, body) in entries {
        let payload = PutPayload::from(body.as_bytes().to_vec());
        memory
            .put(&Path::parse(key).unwrap(), payload)
            .await
            .unwrap();
    }
    ObjectLineageStore::new(Arc::new(memory), Path::parse("store").unwrap())
}

fn record(kind: &str) -> String {
    format!(r#"{{"version":"lineage/v1beta1","kind":"{kind}","spec":{{"name":"x"}}}}"#)
}

#[tokio::test]
async fn in_memory_store_ignores_directory_markers() {
    let run = record("WorkflowRun");
    let out = record("WorkflowOutput");
    let history = format!("2026-11-26 21:44:14 UTC\tcool_name\tsession\tlid://{H}\n");
    let store = memory_store(&[
        ("store", ""),
        ("store/.history", ""),
        (&format!("store/{H}#output"), ""),
        (&format!("store/.history/{H}"), &history),
        (&format!("store/.history/nested/{H}"), "not a history line"),
        ("store/.history/x/.data.json", &run),
        (&format!("store/{H}/.data.json"), &run),
        (&format!("store/{H}#output/.data.json"), &out),
        ("other/ffff/.data.json", &run),
    ])
    .await;

    let runs = store.list_runs().await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        (runs[0].run_name.as_str(), runs[0].lid.to_string()),
        ("cool_name", format!("lid://{H}"))
    );
    let output = Lid::parse(&format!("lid://{H}#output")).unwrap();
    assert_eq!(
        store.find(&query(&[])).await.unwrap(),
        vec![runs[0].lid.clone(), output.clone()]
    );
    assert!(matches!(
        store.get(&output).await.unwrap(),
        Record::WorkflowOutput(_)
    ));
}

#[tokio::test]
async fn store_errors_are_typed() {
    let fixture = common::store_dir("26.10.7");
    let local = ObjectLineageStore::open(fixture.to_str().unwrap()).unwrap();
    let missing = Lid::parse("lid://ffff").unwrap();
    match local.get(&missing).await {
        Err(LineageError::NotFound { target }) => assert_eq!(target, "lid://ffff"),
        other => panic!("{other:?}"),
    }

    for bad in ["garbage", "a\tb\tc", "a\t\tc\td", "a\tb\tc\tnot-a-lid"] {
        let store = memory_store(&[(&format!("store/.history/{H}"), bad)]).await;
        match store.list_runs().await {
            Err(LineageError::InvalidFormat { path, .. }) => assert!(path.ends_with(H), "{path}"),
            other => panic!("{bad}: {other:?}"),
        }
    }
    assert!(
        memory_store(&[])
            .await
            .list_runs()
            .await
            .unwrap()
            .is_empty()
    );

    let store = memory_store(&[(&format!("store/{H}/.data.json"), "{")]).await;
    assert!(matches!(
        store.find(&query(&[])).await,
        Err(LineageError::InvalidFormat { .. })
    ));

    assert!(matches!(
        ObjectLineageStore::open("ftp://host/x"),
        Err(LineageError::Unsupported(_))
    ));
    let nowhere = fixture.join("does-not-exist");
    assert!(matches!(
        ObjectLineageStore::open(nowhere.to_str().unwrap()),
        Err(LineageError::NotFound { .. })
    ));
    assert!(matches!(
        ObjectLineageStore::open_with_options("s3://bucket/store", [("no_such_option", "x")]),
        Err(LineageError::Unsupported(_))
    ));
    assert!(matches!(
        ObjectLineageStore::open_with_options(fixture.to_str().unwrap(), [("aws_region", "x")]),
        Err(LineageError::Unsupported(_))
    ));
    // Explicit options are accepted for the cloud scheme they belong to; nothing is contacted.
    assert!(
        ObjectLineageStore::open_with_options(
            "s3://bucket/store",
            [
                ("aws_endpoint", "http://localhost:1"),
                ("aws_allow_http", "true"),
                ("aws_region", "us-east-1")
            ],
        )
        .is_ok()
    );
}

#[tokio::test]
async fn find_skips_unrepresentable_objects() {
    let run = record("WorkflowRun");
    // `#` inside a path segment would otherwise be read as the fragment separator.
    let store = memory_store(&[
        (&format!("store/{H}/.data.json"), &run),
        (&format!("store/{H}/a#b/.data.json"), &run),
        (&format!("store/{H}/c?d/.data.json"), &run),
    ])
    .await;
    assert_eq!(
        store.find(&query(&[])).await.unwrap(),
        vec![Lid::parse(&format!("lid://{H}")).unwrap()]
    );
}

#[tokio::test]
async fn find_keeps_listing_when_local_objects_are_unrepresentable() {
    let (tmp, _) = common::fixture_copy("26.10.7");
    let dir = tmp.path().join(".lineage");
    let store = ObjectLineageStore::open(dir.to_str().unwrap()).unwrap();
    let body = fs::read(&common::record_files(&dir)[0]).unwrap();
    let before = store.find(&query(&[])).await.unwrap();
    for key in ["ffff/odd#x", "ffff/odd?x"] {
        fs::create_dir_all(dir.join(key)).unwrap();
        fs::write(dir.join(key).join(".data.json"), &body).unwrap();
    }
    assert_eq!(store.find(&query(&[])).await.unwrap(), before);
}
