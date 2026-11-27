//! Fixture helpers shared by the integration tests (`fixtures/lineage/<version>/`).

use std::fs;
use std::path::{Path, PathBuf};

/// `fixtures/lineage` at the repository root.
pub fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/lineage")
}

/// Every fixture version dir name (core versions and `26.10.7-agent`), sorted.
pub fn versions() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixture_root())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert!(names.len() >= 5, "fixtures missing: {names:?}");
    names
}

/// The `.lineage` store of a fixture version.
pub fn store_dir(version: &str) -> PathBuf {
    fixture_root().join(version).join(".lineage")
}

/// Every `.data.json` file below `dir`, sorted.
pub fn record_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(record_files(&path));
        } else if path.file_name().is_some_and(|n| n == ".data.json") {
            out.push(path);
        }
    }
    out.sort();
    out
}
