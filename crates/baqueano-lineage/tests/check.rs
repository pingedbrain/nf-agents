//! `check` against every fixture FileOutput, on a temp copy with restored mtimes.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::fs;
use std::time::SystemTime;

use baqueano_lineage::{
    CheckOptions, CheckStatus, FindQuery, LineageError, LineageStore, ObjectLineageStore, check,
};

#[tokio::test]
async fn fixture_outcomes() {
    for version in common::versions() {
        let (tmp, manifest) = common::fixture_copy(&version);
        let store =
            ObjectLineageStore::open(tmp.path().join(".lineage").to_str().unwrap()).unwrap();
        let opts = CheckOptions {
            path_map: vec![(
                manifest["recorded_root"].as_str().unwrap().to_owned(),
                tmp.path().to_owned(),
            )],
            ..CheckOptions::default()
        };
        let query = |q: &str| FindQuery::parse_args([q]).unwrap();
        let files = store.find(&query("type=FileOutput")).await.unwrap();
        if manifest["kind"] == "core" {
            assert_eq!(files.len(), 10, "{version}");
        }
        for lid in &files {
            let report = check(&store, lid, &opts).await.unwrap();
            assert_eq!(report.status, CheckStatus::Valid, "{version} {report:?}");
        }
        let Some(lid) = files.first() else { continue };
        let local = |r: &baqueano_lineage::CheckReport| {
            tmp.path()
                .join(r.path.strip_prefix("/nf-agents-fixture/").unwrap())
        };
        let report = check(&store, lid, &opts).await.unwrap();
        let file = local(&report);

        let restored = fs::metadata(&file).unwrap().modified().unwrap();
        let handle = fs::File::options().append(true).open(&file).unwrap();
        handle.set_modified(SystemTime::now()).unwrap();
        let report = check(&store, lid, &opts).await.unwrap();
        assert_eq!(report.status, CheckStatus::Mismatch, "{version} mtime");
        assert_eq!(report.expected.as_deref().map(str::len), Some(32));
        assert_ne!(report.expected, report.actual);

        use std::io::Write;
        (&handle).write_all(b"x").unwrap();
        handle.set_modified(restored).unwrap();
        let report = check(&store, lid, &opts).await.unwrap();
        assert_eq!(report.status, CheckStatus::Mismatch, "{version} size");

        fs::remove_file(&file).unwrap();
        let report = check(&store, lid, &opts).await.unwrap();
        assert_eq!(report.status, CheckStatus::FileMissing, "{version} missing");

        let task = store.find(&query("type=TaskRun")).await.unwrap();
        let err = check(&store, &task[0], &opts).await.unwrap_err();
        assert!(matches!(err, LineageError::UnexpectedKind { .. }), "{err}");
    }
}
