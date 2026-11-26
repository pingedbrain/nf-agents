//! Key stability against the 8 real agent requests captured in the lab (Nextflow 26.10.7).

// Helpers outside #[test] fns are not covered by clippy.toml's allow-*-in-tests.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::path::PathBuf;

use nf_llm_replay::normalize::Normalizer;
use serde_json::Value;

/// Golden keys (cross-checked with an independent Python implementation of the recipe).
/// Changing any of them breaks every existing cassette (bump the cassette `v`).
const GOLDEN: [(&str, &str); 8] = [
    (
        "langchain4j-json-schema.json",
        "0907b91e8a4e65d041388cb57312f688b06c0dc0ea0e34a4167a4d1b7165ace3",
    ),
    (
        "langchain4j-plain.json",
        "7fd65e9a4f625adaaf6d7c332c7ba56e848c8093b36f00ca1c388783fef7a24a",
    ),
    (
        "langchain4j-tools.json",
        "e06b407442edc405bed723a3de0d7c8daf07cd41c120c36b6c75178ca8ca08d6",
    ),
    (
        "pi-anthropic-plain.json",
        "de6b716e600244bd3cf1f1317d84bb73bf677df05d87b44278b6b062cebb51ac",
    ),
    (
        "pi-openai-final-answer-retry.json",
        "62e38d419a60fbacdd5abe25403f01ed831c79b6f7445ea60340de9be35047cb",
    ),
    (
        "pi-openai-final-answer.json",
        "dd73296c5564a2c3e7bd2c2e7d0d47108cc6ddd83636ff69814d350cde593f2d",
    ),
    (
        "pi-openai-plain.json",
        "7bd726161b5dacf590c0395ad537cb3116d2758c5950669e2e35e14bbf30d2f1",
    ),
    (
        "pi-openai-tool.json",
        "e5d534d63c9845ead4d133a2252a3ce1572fb7c2b5a92f27af3d39f245306e58",
    ),
];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/captures")
}

/// Returns (method, path, body) of a fixture.
fn capture(name: &str) -> (String, String, Value) {
    let text = std::fs::read_to_string(dir().join(name)).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    (
        v["method"].as_str().unwrap().to_owned(),
        v["path"].as_str().unwrap().to_owned(),
        v["body"].clone(),
    )
}

fn key(method: &str, path: &str, body: &Value) -> (String, String) {
    let out = Normalizer::new()
        .unwrap()
        .normalize(method, path, &serde_json::to_vec(body).unwrap())
        .unwrap();
    (out.key, out.canonical)
}

#[test]
fn capture_keys_are_stable() {
    let mut on_disk: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    on_disk.sort();
    let listed: Vec<&str> = GOLDEN.iter().map(|(name, _)| *name).collect();
    assert_eq!(on_disk, listed, "every capture fixture has a golden key");

    for (name, golden) in GOLDEN {
        let (method, path, body) = capture(name);
        assert_eq!(key(&method, &path, &body).0, golden, "{name}");
    }
}

#[test]
fn pi_requests_differing_only_in_cwd_and_cache_key_share_a_key() {
    for name in ["pi-openai-plain.json", "pi-anthropic-plain.json"] {
        let (method, path, body) = capture(name);
        let text = serde_json::to_string(&body).unwrap();
        assert!(text.contains("<cwd>\\n/lab/"), "{name} has a work dir");

        let mut other: Value =
            serde_json::from_str(&text.replace("/lab/agent-capture/runs/", "/other/run/")).unwrap();
        if other.get("prompt_cache_key").is_some() {
            other["prompt_cache_key"] = "another-session".into();
        }
        assert_ne!(other, body);

        let (k1, canonical) = key(&method, &path, &body);
        let (k2, _) = key(&method, &path, &other);
        assert_eq!(k1, k2, "{name}");
        assert!(canonical.contains("<volatile/>"), "{name}");
        assert!(!canonical.contains("/lab/"), "{name}");
        assert!(!canonical.contains("prompt_cache_key"), "{name}");
    }
}
