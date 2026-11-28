//! CLI contract: `list`, `view`, `find`, `diff` and `check` against the `nextflow lineage` output
//! recorded in `fixtures/lineage/<version>/oracle/<case>.{args,out,err,exit}`. Failures are
//! collected and reported as `<version> <case>: …`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::time::SystemTime;

use baqueano_lineage::{
    CheckOptions, CheckStatus, FindQuery, Lid, LineageError, LineageStore, ObjectLineageStore,
    Record, check, diff_raw, view,
};
use serde_json::{Map, Number, Value};

/// One recorded CLI invocation; `text` is stdout followed by stderr.
struct Case {
    version: String,
    name: String,
    args: Vec<String>,
    out: String,
    text: String,
    exit: i32,
}

impl Case {
    fn fail(&self, msg: impl std::fmt::Display) -> String {
        format!("{} {}: {msg}", self.version, self.name)
    }
}

/// Every oracle case whose name starts with `prefix`, over all fixture versions.
fn cases(prefix: &str) -> Vec<Case> {
    let mut out = Vec::new();
    for version in common::versions() {
        let dir = common::fixture_root().join(&version).join("oracle");
        let read = |name: &str, ext: &str| fs::read_to_string(dir.join(format!("{name}.{ext}")));
        let mut names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| {
                let name = e.unwrap().file_name().into_string().unwrap();
                name.strip_suffix(".args").map(str::to_owned)
            })
            .filter(|n| n.starts_with(prefix))
            .collect();
        names.sort();
        for name in names {
            let stdout = read(&name, "out").unwrap();
            out.push(Case {
                args: serde_json::from_str(&read(&name, "args").unwrap()).unwrap(),
                text: format!("{stdout}{}", read(&name, "err").unwrap_or_default()),
                exit: read(&name, "exit").unwrap().trim().parse().unwrap(),
                out: stdout,
                version: version.clone(),
                name,
            });
        }
    }
    assert!(!out.is_empty(), "no oracle cases for {prefix}");
    out
}

fn store(version: &str) -> ObjectLineageStore {
    ObjectLineageStore::open(common::store_dir(version).to_str().unwrap()).unwrap()
}

fn report(failures: Vec<String>) {
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn contract_list() {
    let mut failures = Vec::new();
    for case in cases("list") {
        let expected: Vec<Vec<String>> = case
            .out
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.split('\t').map(|c| c.trim().to_owned()).collect())
            .collect();
        let ours: Vec<Vec<String>> = store(&case.version)
            .list_runs()
            .await
            .unwrap()
            .into_iter()
            .map(|r| vec![r.timestamp, r.run_name, r.session_id, r.lid.to_string()])
            .collect();
        if ours != expected {
            failures.push(case.fail(format!("{ours:?} != {expected:?}")));
        }
    }
    report(failures);
}

/// The oracle error a case expects, by the CLI message.
fn expected_error(case: &Case) -> Option<&'static str> {
    let text = &case.text;
    if text.contains("Identifier is not a lineage URL") {
        Some("InvalidLid")
    } else if text.contains("doesn't exist in the lineage model") {
        Some("InvalidQuery")
    } else if case.out.starts_with("Error loading") && text.contains("not found") {
        Some("NotFound")
    } else {
        assert_eq!(case.exit, 0, "{} {}: unclassified", case.version, case.name);
        None
    }
}

fn error_name(err: &LineageError) -> &'static str {
    match err {
        LineageError::InvalidLid { .. } => "InvalidLid",
        LineageError::InvalidQuery { .. } => "InvalidQuery",
        LineageError::NotFound { .. } => "NotFound",
        _ => "other",
    }
}

/// Flat (25.04) envelope → `spec` envelope at the top level; every number as f64; object
/// members that are `null` dropped (null ≡ absent).
fn normalize(value: Value) -> Value {
    let value = match value {
        Value::Object(mut m) if m.contains_key("kind") && !m.contains_key("spec") => {
            let mut wrapped = Map::new();
            for key in ["version", "kind"] {
                wrapped.insert(key.to_owned(), m.remove(key).unwrap_or_default());
            }
            wrapped.insert("spec".to_owned(), Value::Object(m));
            Value::Object(wrapped)
        }
        v => v,
    };
    fn deep(value: Value) -> Value {
        match value {
            Value::Object(m) => Value::Object(
                m.into_iter()
                    .filter(|(_, v)| !v.is_null())
                    .map(|(k, v)| (k, deep(v)))
                    .collect(),
            ),
            Value::Array(items) => Value::Array(items.into_iter().map(deep).collect()),
            Value::Number(n) => n
                .as_f64()
                .and_then(Number::from_f64)
                .map_or(Value::Null, Value::Number),
            v => v,
        }
    }
    deep(value)
}

#[tokio::test]
async fn contract_view() {
    let mut failures = Vec::new();
    for case in cases("view-") {
        let store = store(&case.version);
        let ours = match Lid::parse(&case.args[2]) {
            Ok(lid) => view(&store, &lid).await,
            Err(e) => Err(e),
        };
        match (expected_error(&case), ours) {
            (Some(kind), Err(e)) if error_name(&e) == kind => {}
            (None, Ok(value)) => {
                let expected = normalize(serde_json::from_str(&case.out).unwrap());
                if normalize(value.clone()) != expected {
                    failures.push(case.fail(format!("{value:#}\n!=\n{expected:#}")));
                }
            }
            (expected, ours) => failures.push(case.fail(format!("{expected:?} vs {ours:?}"))),
        }
    }
    report(failures);
}

#[tokio::test]
async fn contract_find() {
    let mut failures = Vec::new();
    for case in cases("find-") {
        let ours = match FindQuery::parse_args(&case.args[2..]) {
            Ok(query) => store(&case.version).find(&query).await,
            Err(e) => Err(e),
        };
        match ours {
            Err(e) if case.out.starts_with("Error searching for") => {
                if !case.out.contains(&e.to_string()) {
                    failures.push(case.fail(format!("message '{e}' not in '{}'", case.out)));
                }
            }
            Ok(lids) if !case.out.starts_with("Error") => {
                let expected: BTreeSet<String> = serde_json::from_str(&case.out)
                    .unwrap_or_else(|_| case.out.lines().map(str::to_owned).collect());
                let expected: BTreeSet<String> =
                    expected.into_iter().filter(|l| !l.is_empty()).collect();
                let ours: BTreeSet<String> = lids.iter().map(Lid::to_string).collect();
                if ours != expected {
                    failures.push(case.fail(format!("{ours:?} != {expected:?}")));
                }
            }
            ours => failures.push(case.fail(format!("{ours:?} vs '{}'", case.out))),
        }
    }
    report(failures);
}

/// The trimmed `+`/`-` lines after the 3-line header, Gson escapes undone, sorted. A trailing
/// `,` is dropped: older models end a record before fields we always emit (`moduleId`).
fn changed_lines(diff: &str) -> Vec<String> {
    let mut lines: Vec<String> = diff
        .lines()
        .skip(3)
        .filter(|l| l.starts_with(['+', '-']))
        .map(|l| {
            let (sign, rest) = l.split_at(1);
            let gson = [
                ("<", "3c"),
                (">", "3e"),
                ("&", "26"),
                ("=", "3d"),
                ("'", "27"),
            ];
            let rest = gson.iter().fold(rest.to_owned(), |s, (c, hex)| {
                s.replace(&format!("\\u00{hex}"), c)
            });
            format!("{sign}{}", rest.trim().trim_end_matches(','))
        })
        .collect();
    lines.sort();
    lines
}

#[tokio::test]
async fn contract_diff() {
    let mut failures = Vec::new();
    for case in cases("diff-") {
        let (a, b) = (
            Lid::parse(&case.args[2]).unwrap(),
            Lid::parse(&case.args[3]).unwrap(),
        );
        let ours = diff_raw(&store(&case.version), &a, &b).await.unwrap();
        let header = |s: &str| s.lines().take(3).collect::<Vec<_>>().join("\n");
        if header(&ours) != header(&case.out) {
            failures.push(case.fail(format!("header {}", header(&ours))));
        }
        if changed_lines(&ours) != changed_lines(&case.out) {
            failures.push(case.fail(format!(
                "changed lines {:#?} != {:#?}",
                changed_lines(&ours),
                changed_lines(&case.out)
            )));
        }
    }
    report(failures);
}

#[tokio::test]
async fn contract_check() {
    let mut failures = Vec::new();
    for case in cases("check-") {
        // A fresh copy per case: the tamper cases change data files like the generator did.
        let (tmp, manifest) = common::fixture_copy(&case.version);
        let store =
            ObjectLineageStore::open(tmp.path().join(".lineage").to_str().unwrap()).unwrap();
        let root = manifest["recorded_root"].as_str().unwrap().to_owned();
        let opts = CheckOptions {
            path_map: vec![(root.clone(), tmp.path().to_owned())],
            ..CheckOptions::default()
        };
        let lid = Lid::parse(&case.args[2]).unwrap();
        if let Ok(Record::FileOutput(f)) = store.get(&lid).await {
            let path = f.path.unwrap();
            let local = tmp
                .path()
                .join(path.strip_prefix(&format!("{root}/")).unwrap());
            let file = fs::File::options().append(true).open(local).unwrap();
            if case.name.ends_with("-tampered") {
                use std::io::Write;
                (&file).write_all(b"tampered\n").unwrap();
            }
            if case.name.ends_with("-tampered") || case.name.ends_with("-mtime-only") {
                file.set_modified(SystemTime::now()).unwrap();
            }
        }
        let expected = if case.exit == 0 {
            "Valid"
        } else if case.text.contains("does not match") {
            "Mismatch"
        } else if case.text.contains("Unexpected lineage object type") {
            "UnexpectedKind"
        } else {
            panic!("{} {}: unclassified", case.version, case.name)
        };
        let ours = match check(&store, &lid, &opts).await {
            Ok(r) if r.status == CheckStatus::Valid => "Valid",
            Ok(r) if r.status == CheckStatus::Mismatch => "Mismatch",
            Err(LineageError::UnexpectedKind { .. }) => "UnexpectedKind",
            other => {
                failures.push(case.fail(format!("{other:?}")));
                continue;
            }
        };
        if ours != expected {
            failures.push(case.fail(format!("{ours} != {expected}")));
        }
    }
    report(failures);
}
