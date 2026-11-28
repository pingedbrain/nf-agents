//! Fixture helpers shared by the integration tests (`fixtures/lineage/<version>/`).
// Each test crate uses a subset of these helpers.
#![allow(dead_code)]

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

/// A temp copy of a fixture version with the data files' mtimes restored from `manifest.json`
/// (git does not keep them), so recorded checksums verify. Returns the copy and the manifest.
pub fn fixture_copy(version: &str) -> (tempfile::TempDir, serde_json::Value) {
    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let path = entry.unwrap().path();
            let target = to.join(path.file_name().unwrap());
            if path.is_dir() {
                copy_dir(&path, &target);
            } else {
                fs::copy(&path, &target).unwrap();
            }
        }
    }
    let tmp = tempfile::tempdir().unwrap();
    let source = fixture_root().join(version);
    copy_dir(&source, tmp.path());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(source.join("manifest.json")).unwrap()).unwrap();
    for file in manifest["files"].as_array().unwrap() {
        let mtime = std::time::UNIX_EPOCH
            + std::time::Duration::from_millis(file["mtime_ms"].as_u64().unwrap());
        let path = tmp.path().join(file["path"].as_str().unwrap());
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
    }
    (tmp, manifest)
}
