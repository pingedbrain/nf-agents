//! Semantic `diff`: lineage records compared with volatile fields ignored.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::{FindQuery, Lid, LineageError, LineageStore, Record};

/// The semantic differences between two lineage records.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SemanticDiff {
    /// The first record.
    pub a: Lid,
    /// The second record.
    pub b: Lid,
    /// The changed projection keys, sorted by `path`.
    pub changes: Vec<Change>,
}

/// One changed value; `None` means absent or `null`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Change {
    /// The projection key, e.g. `params.greeting` or `tasks.ALIGN.container`.
    pub path: String,
    /// The value in the first record.
    pub a: Option<Value>,
    /// The value in the second record.
    pub b: Option<Value>,
}

/// Projection key → value (LID hashes masked, nulls dropped).
type Projection = BTreeMap<String, Value>;

/// Never compared for unknown kinds: they differ between any two runs.
const VOLATILE: &[&str] = &[
    "sessionId",
    "createdAt",
    "modifiedAt",
    "workflowRun",
    "taskRun",
];
/// Per-task-name fields of a run-level diff.
const TASK_FIELDS: &str =
    "codeChecksum container conda spack architecture moduleId runner model resolvedModel";

/// The changes between two records over a whitelist projection per kind (identity, code, tools,
/// inputs, outputs, size, labels), ignoring session IDs, run names, timestamps, command lines,
/// output dirs and LID hashes. Records of different kinds give one `kind` change.
pub fn diff_records(a: &Record, b: &Record) -> Vec<Change> {
    if a.kind() != b.kind() {
        return vec![Change {
            path: "kind".to_owned(),
            a: Some(a.kind().into()),
            b: Some(b.kind().into()),
        }];
    }
    compare(&project(a), &project(b))
}

/// [`diff_records`] of two stored records; two WorkflowRuns also compare their tasks
/// (`tasks.<name>.<field>`, `tasks.<name>.count`) and published outputs (`outputs.<name>`).
///
/// # Errors
/// [`LineageError::NotFound`] for a missing record, and store errors.
pub async fn diff(
    store: &dyn LineageStore,
    a: &Lid,
    b: &Lid,
) -> Result<SemanticDiff, LineageError> {
    let (ra, rb) = (store.get(a).await?, store.get(b).await?);
    let changes = match (&ra, &rb) {
        (Record::WorkflowRun(_), Record::WorkflowRun(_)) => compare(
            &run_projection(store, a, &ra).await?,
            &run_projection(store, b, &rb).await?,
        ),
        _ => diff_records(&ra, &rb),
    };
    Ok(SemanticDiff {
        a: a.clone(),
        b: b.clone(),
        changes,
    })
}

/// `nextflow lineage diff`: a `diff --git k1 k2` / `--- k1` / `+++ k2` header (keys are the LIDs
/// without `lid://`) and a unified diff (3 context lines) of the pretty-printed records, in the
/// canonical `{"version", "kind", "spec"}` envelope for every Nextflow version.
///
/// # Errors
/// [`LineageError::NotFound`] for a missing record, and store errors.
pub async fn diff_raw(store: &dyn LineageStore, a: &Lid, b: &Lid) -> Result<String, LineageError> {
    // Serializing a `Value` cannot fail.
    let pretty = |r: &Record| serde_json::to_string_pretty(&r.to_json()).unwrap_or_default() + "\n";
    let (ta, tb) = (pretty(&store.get(a).await?), pretty(&store.get(b).await?));
    let key = |lid: &Lid| lid.to_string().trim_start_matches("lid://").to_owned();
    let (ka, kb) = (key(a), key(b));
    let hunks = similar::TextDiff::from_lines(&ta, &tb)
        .unified_diff()
        .to_string();
    Ok(format!("diff --git {ka} {kb}\n--- {ka}\n+++ {kb}\n{hunks}"))
}

fn compare(a: &Projection, b: &Projection) -> Vec<Change> {
    let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    keys.into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .map(|k| Change {
            path: k.clone(),
            a: a.get(k).cloned(),
            b: b.get(k).cloned(),
        })
        .collect()
}

/// The whitelisted fields of a record (dot paths into `spec()`); list items are
/// keyed by their `name` (or `path`, else `#<index>`) and contribute their `value` (or
/// `checksum`).
fn project(record: &Record) -> Projection {
    let spec = record.spec();
    let (fields, lists) = match record {
        Record::WorkflowRun(_) => (
            "workflow.repository workflow.commitId",
            "params workflow.scriptFiles",
        ),
        Record::TaskRun(_) => (
            "name codeChecksum script container conda spack architecture moduleId",
            "input binEntries",
        ),
        Record::AgentRun(_) => (
            "name codeChecksum runner model resolvedModel instruction goal promptTemplate \
             maxIterations outputSchema tools skills container moduleId",
            "input",
        ),
        Record::FileOutput(_) => ("size labels source", ""),
        Record::TaskOutput(_) | Record::WorkflowOutput(_) => ("labels", "output"),
        _ => ("", ""),
    };
    let field = |key: &str| spec.pointer(&format!("/{}", key.replace('.', "/")));
    let mut out = Projection::new();
    for key in fields.split_whitespace() {
        put(&mut out, key.to_owned(), field(key));
    }
    for list in lists.split_whitespace() {
        let items = field(list).and_then(Value::as_array);
        for (i, item) in items.into_iter().flatten().enumerate() {
            let label = item
                .get("name")
                .or_else(|| item.get("path"))
                .and_then(Value::as_str);
            let mut k = format!("{list}.{}", label.map_or(format!("#{i}"), str::to_owned));
            if out.contains_key(&k) {
                k = format!("{list}.#{i}");
            }
            put(
                &mut out,
                k,
                item.get("value").or_else(|| item.get("checksum")),
            );
        }
    }
    match record {
        Record::WorkflowRun(_) => {
            put(
                &mut out,
                "nextflow.version".to_owned(),
                spec.pointer("/metadata/nextflow/version"),
            );
        }
        Record::Unknown(_) => {
            for (k, v) in spec.as_object().into_iter().flatten() {
                if !VOLATILE.contains(&k.as_str()) {
                    put(&mut out, k.clone(), Some(v));
                }
            }
        }
        _ => {}
    }
    out
}

fn put(out: &mut Projection, key: String, value: Option<&Value>) {
    if let Some(v) = value.filter(|v| !v.is_null()) {
        out.insert(key, masked(v));
    }
}

/// A WorkflowRun's projection plus its tasks (by name) and published outputs.
async fn run_projection(
    store: &dyn LineageStore,
    lid: &Lid,
    record: &Record,
) -> Result<Projection, LineageError> {
    let mut out = project(record);
    let run = format!("lid://{}", lid.hash());
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut values: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for found in store
        .find(&FindQuery::new([("workflowRun", run.as_str())])?)
        .await?
    {
        let task = store.get(&found).await?;
        if !matches!(task, Record::TaskRun(_) | Record::AgentRun(_)) {
            continue;
        }
        let p = project(&task);
        let name = p
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        for field in TASK_FIELDS.split_whitespace() {
            if let Some(v) = p.get(field) {
                values
                    .entry(format!("tasks.{name}.{field}"))
                    .or_default()
                    .push(v.clone());
            }
        }
        *counts.entry(name).or_default() += 1;
    }
    for (name, n) in counts {
        out.insert(format!("tasks.{name}.count"), n.into());
    }
    for (k, mut vs) in values {
        vs.sort_by_key(Value::to_string);
        vs.dedup();
        let v = match <[Value; 1]>::try_from(vs) {
            Ok([one]) => one,
            Err(vs) => Value::Array(vs),
        };
        out.insert(k, v);
    }
    match store.get(&Lid::parse(&format!("{run}#output"))?).await {
        Ok(output) => {
            for (k, v) in project(&output) {
                if let Some(name) = k.strip_prefix("output.") {
                    out.insert(format!("outputs.{name}"), v);
                }
            }
        }
        Err(LineageError::NotFound { .. }) => {}
        Err(e) => return Err(e),
    }
    Ok(out)
}

/// `v` with every `lid://<hash>` in its strings replaced by `lid://*`.
fn masked(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(mask(s)),
        Value::Array(items) => items.iter().map(masked).collect(),
        Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), masked(v))).collect(),
        other => other.clone(),
    }
}

fn mask(s: &str) -> String {
    const PREFIX: &str = "lid://";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some((head, tail)) = rest.split_once(PREFIX) {
        out.push_str(head);
        out.push_str("lid://*");
        rest = tail.trim_start_matches(|c: char| c.is_ascii_hexdigit());
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn record(kind: &str, spec: serde_json::Value) -> Record {
        let json = json!({"version": "lineage/v1beta1", "kind": kind, "spec": spec});
        Record::parse(json.to_string().as_bytes(), "test").unwrap()
    }

    #[test]
    fn projection_ignores_volatile_fields_and_lid_hashes() {
        let task = |session: &str, hash: &str, container: &str| {
            record(
                "TaskRun",
                json!({
                    "sessionId": session, "name": "T", "workflowRun": format!("lid://{hash}"),
                    "container": container, "eval": null,
                    "input": [
                        {"type": "path", "name": "reads", "value": [format!("lid://{hash}/r.fq")]},
                        {"type": "val", "name": null, "value": 1}
                    ]
                }),
            )
        };
        let a = task("s1", "aaaa", "c:1");
        assert!(diff_records(&a, &task("s2", "bbbb", "c:1")).is_empty());
        let changes = diff_records(&a, &task("s1", "aaaa", "c:2"));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "container");
        assert_eq!(project(&a)["input.reads"], json!(["lid://*/r.fq"]));
        assert_eq!(project(&a)["input.#1"], json!(1));

        // Unknown kinds: top-level spec keys minus the volatile ones; null is absent.
        let unknown = |v: i64, at: &str| {
            record(
                "Future",
                json!({"x": v, "createdAt": at, "taskRun": at, "y": null}),
            )
        };
        assert!(diff_records(&unknown(1, "t1"), &unknown(1, "t2")).is_empty());
        let absent = record("Future", json!({"x": 1}));
        assert!(diff_records(&unknown(1, "t1"), &absent).is_empty());
        let changes = diff_records(&unknown(1, "t1"), &unknown(2, "t1"));
        assert_eq!(changes[0].path, "x");
        assert_eq!(
            mask("lid://ab12/x lid:// lid://zz"),
            "lid://*/x lid://* lid://*zz"
        );
    }
}
