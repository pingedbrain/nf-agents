//! Every record of every fixture store parses; one insta snapshot per version.
// Integration tests: panicking on unexpected fixture state is the failure mode we want.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::fs;

use baqueano_lineage::Record;
use regex::Regex;

#[test]
fn every_fixture_record_parses() {
    for version in common::versions() {
        let files = common::record_files(&common::store_dir(&version));
        assert!(!files.is_empty(), "{version}: no records");
        for file in files {
            let origin = file.display().to_string();
            let record = Record::parse(&fs::read(&file).unwrap(), &origin)
                .unwrap_or_else(|e| panic!("{version}: {e}"));
            assert!(
                !matches!(record, Record::Unknown(_)),
                "{origin}: unknown kind"
            );
        }
    }
}

/// Masks what changes on every regeneration (hashes, work dirs, UUIDs, timestamps, run names).
fn mask(json: &str, run_names: &[String]) -> String {
    let mut out = json.to_owned();
    for name in run_names {
        out = out.replace(name.as_str(), "[RUN_NAME]");
    }
    for (re, to) in [
        (r"[0-9a-f]{2}/[0-9a-f]{30}", "[WORKDIR]"),
        (r"[0-9a-f]{32}", "[HASH]"),
        (
            r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
            "[UUID]",
        ),
        (r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d+)?Z", "[TIME]"),
    ] {
        out = Regex::new(re).unwrap().replace_all(&out, to).into_owned();
    }
    out
}

#[test]
fn fixture_snapshots() {
    for version in common::versions() {
        let store = common::store_dir(&version);
        let run_names: Vec<String> = fs::read_dir(store.join(".history"))
            .unwrap()
            .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
            .map(|line| line.split('\t').nth(1).unwrap().to_owned())
            .collect();
        // Masked before sorting, so the order survives a regeneration.
        let mut records: Vec<String> = common::record_files(&store)
            .iter()
            .map(|file| {
                let record = Record::parse(&fs::read(file).unwrap(), "fixture").unwrap();
                mask(&record.to_json().to_string(), &run_names)
            })
            .collect();
        records.sort();
        let records: Vec<serde_json::Value> = records
            .iter()
            .map(|r| serde_json::from_str(r).unwrap())
            .collect();
        insta::assert_json_snapshot!(format!("records-{version}"), records);
    }
}
