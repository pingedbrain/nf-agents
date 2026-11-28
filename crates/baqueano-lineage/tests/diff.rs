//! Semantic `diff` over the fixture runs (RUN1 `hello`, RUN2 `bonjour`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use baqueano_lineage::{FindQuery, Lid, LineageStore, ObjectLineageStore, diff, diff_raw};
use serde_json::json;

fn core_versions() -> Vec<String> {
    common::versions()
        .into_iter()
        .filter(|v| !v.ends_with("-agent"))
        .collect()
}

async fn runs(store: &ObjectLineageStore) -> (Lid, Lid) {
    let runs = store.list_runs().await.unwrap();
    assert_eq!(runs.len(), 2);
    (runs[0].lid.clone(), runs[1].lid.clone())
}

async fn generate_task(store: &ObjectLineageStore, run: &Lid) -> Lid {
    let run = format!("workflowRun={run}");
    let query = FindQuery::parse_args(["type=TaskRun", "name=GENERATE", run.as_str()]).unwrap();
    store.find(&query).await.unwrap().remove(0)
}

fn paths(diff: &baqueano_lineage::SemanticDiff) -> Vec<&str> {
    diff.changes.iter().map(|c| c.path.as_str()).collect()
}

#[tokio::test]
async fn minimal_runs_differ_only_in_greeting() {
    for version in core_versions() {
        let store =
            ObjectLineageStore::open(common::store_dir(&version).to_str().unwrap()).unwrap();
        let (run1, run2) = runs(&store).await;
        let d = diff(&store, &run1, &run2).await.unwrap();
        assert_eq!(
            paths(&d),
            ["params.greeting"],
            "{version}: {:#?}",
            d.changes
        );
        assert_eq!(d.changes[0].a, Some(json!("hello")));
        assert_eq!(d.changes[0].b, Some(json!("bonjour")));
        assert!(diff(&store, &run1, &run1).await.unwrap().changes.is_empty());
    }
}

#[tokio::test]
async fn task_pairs_and_kind_mismatch() {
    for version in core_versions() {
        let store =
            ObjectLineageStore::open(common::store_dir(&version).to_str().unwrap()).unwrap();
        let (run1, run2) = runs(&store).await;
        let (task1, task2) = (
            generate_task(&store, &run1).await,
            generate_task(&store, &run2).await,
        );
        let d = diff(&store, &task1, &task2).await.unwrap();
        assert_eq!(
            paths(&d),
            ["input.greeting", "script"],
            "{version}: {:#?}",
            d.changes
        );

        let d = diff(&store, &run1, &task1).await.unwrap();
        assert_eq!(paths(&d), ["kind"]);
        assert_eq!(d.changes[0].a, Some(json!("WorkflowRun")));
        assert_eq!(d.changes[0].b, Some(json!("TaskRun")));
    }
}

#[tokio::test]
async fn raw_diff_has_cli_header_and_record_lines() {
    for version in core_versions() {
        let store =
            ObjectLineageStore::open(common::store_dir(&version).to_str().unwrap()).unwrap();
        let (run1, run2) = runs(&store).await;
        let raw = diff_raw(&store, &run1, &run2).await.unwrap();
        let (k1, k2) = (run1.hash(), run2.hash());
        let header = format!("diff --git {k1} {k2}\n--- {k1}\n+++ {k2}\n@@ ");
        assert!(raw.starts_with(&header), "{version}: {raw}");
        let changed = |sign: char| -> Vec<&str> {
            raw.lines()
                .skip(3)
                .filter_map(|l| l.strip_prefix(sign))
                .map(str::trim)
                .collect()
        };
        assert!(
            changed('-').contains(&r#""value": "hello""#),
            "{version}: {raw}"
        );
        assert!(
            changed('+').contains(&r#""value": "bonjour""#),
            "{version}: {raw}"
        );
        let context = raw.lines().skip(3).filter(|l| l.starts_with(' ')).count();
        assert!(context >= 6, "{version}: context lines");

        let same = diff_raw(&store, &run1, &run1).await.unwrap();
        assert_eq!(same.lines().count(), 3, "{version}: {same}");
    }
}
