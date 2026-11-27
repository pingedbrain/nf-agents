//! `provenance` over the fixture runs: published file → tasks → pipeline input.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::collections::BTreeSet;

use std::sync::Arc;

use nf_lineage::object_store::memory::InMemory;
use nf_lineage::object_store::path::Path;
use nf_lineage::object_store::{ObjectStoreExt, PutPayload};
use nf_lineage::{Lid, LineageError, ObjectLineageStore, ProvenanceInput, provenance};

fn core_versions() -> Vec<String> {
    common::versions()
        .into_iter()
        .filter(|v| !v.ends_with("-agent"))
        .collect()
}

fn id(version: &str, name: &str) -> String {
    let path = common::fixture_root().join(version).join("oracle/ids.json");
    let ids: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    ids[name].as_str().unwrap().to_owned()
}

fn task_names(p: &nf_lineage::Provenance) -> Vec<&str> {
    p.nodes
        .iter()
        .filter(|n| n.kind == "TaskRun")
        .filter_map(|n| n.name.as_deref())
        .collect()
}

#[tokio::test]
async fn published_file_reaches_pipeline_input() {
    for version in core_versions() {
        let store =
            ObjectLineageStore::open(common::store_dir(&version).to_str().unwrap()).unwrap();
        let file = Lid::parse(&format!("{}/summary.txt", id(&version, "RUN1"))).unwrap();
        let p = provenance(&store, &file).await.unwrap();
        assert_eq!(p.target, file);
        assert_eq!(p.nodes[0].lid, file, "{version}: target first");
        assert_eq!(p.nodes[0].kind, "FileOutput");
        assert_eq!(
            task_names(&p),
            ["SUMMARIZE", "TRANSFORM", "GENERATE"],
            "{version}: {p:#?}"
        );
        let unique: BTreeSet<&Lid> = p.nodes.iter().map(|n| &n.lid).collect();
        assert_eq!(unique.len(), p.nodes.len(), "{version}: revisited");

        let inputs = || p.nodes.iter().flat_map(|n| &n.inputs);
        assert!(
            inputs().any(
                |i| matches!(i, ProvenanceInput::External { path, checksum, .. }
                if path.ends_with("seed.txt") && checksum.is_some())
            ),
            "{version}: {p:#?}"
        );
        assert!(
            inputs().any(|i| matches!(i, ProvenanceInput::Value { name, value }
                if name.as_deref() == Some("greeting") && value == "hello")),
            "{version}: {p:#?}"
        );
        // Every lineage input was visited.
        for input in inputs() {
            if let ProvenanceInput::Lineage { lid, .. } = input {
                assert!(unique.contains(lid), "{version}: {lid} not visited");
            }
        }
    }
}

#[tokio::test]
async fn task_file_stops_at_its_task_and_runs_are_rejected() {
    for version in core_versions() {
        let store =
            ObjectLineageStore::open(common::store_dir(&version).to_str().unwrap()).unwrap();
        let task = id(&version, "TASK_GENERATE_RUN1");
        let raw = Lid::parse(&format!("{task}/raw.txt")).unwrap();
        let p = provenance(&store, &raw).await.unwrap();
        assert_eq!(task_names(&p), ["GENERATE"], "{version}: {p:#?}");

        let run = Lid::parse(&id(&version, "RUN1")).unwrap();
        let err = provenance(&store, &run).await.unwrap_err();
        assert!(
            matches!(&err, LineageError::UnexpectedKind { found, .. } if found == "WorkflowRun"),
            "{err}"
        );
        let missing = Lid::parse("lid://00000000000000000000000000000000/x.txt").unwrap();
        let err = provenance(&store, &missing).await.unwrap_err();
        assert!(matches!(err, LineageError::NotFound { .. }), "{err}");
    }
}

#[tokio::test]
async fn cycles_are_visited_once() {
    // a.txt comes from task t, whose input is a.txt again.
    let memory = InMemory::new();
    for (key, body) in [
        (
            "aa/a.txt/.data.json",
            r#"{"version":"lineage/v1beta1","kind":"FileOutput","spec":{"source":"lid://bb","taskRun":"lid://bb"}}"#,
        ),
        (
            "bb/.data.json",
            r#"{"version":"lineage/v1beta1","kind":"TaskRun","spec":{"name":"T","input":[{"type":"path","name":"x","value":"lid://aa/a.txt"}]}}"#,
        ),
    ] {
        let payload = PutPayload::from(body.as_bytes().to_vec());
        memory
            .put(&Path::parse(key).unwrap(), payload)
            .await
            .unwrap();
    }
    let store = ObjectLineageStore::new(Arc::new(memory), Path::default());
    let p = provenance(&store, &Lid::parse("lid://aa/a.txt").unwrap())
        .await
        .unwrap();
    let lids: Vec<String> = p.nodes.iter().map(|n| n.lid.to_string()).collect();
    assert_eq!(lids, ["lid://aa/a.txt", "lid://bb"]);
    assert_eq!(p.nodes[0].inputs.len(), 1, "source == taskRun kept once");
}
