//! `provenance`: the tasks and inputs a data file was produced from.

use std::collections::{BTreeSet, VecDeque};

use serde::Serialize;
use serde_json::Value;

use crate::model::Parameter;
use crate::{Checksum, Lid, LineageError, LineageStore, Record};

/// The provenance graph of a data file, breadth-first from the file itself.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Provenance {
    /// The file asked about.
    pub target: Lid,
    /// Every record reached, the target first, each once.
    pub nodes: Vec<ProvenanceNode>,
}

/// One lineage record of a provenance graph.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProvenanceNode {
    /// The record's LID.
    pub lid: Lid,
    /// The record kind, e.g. `TaskRun`.
    pub kind: String,
    /// The task or agent name; the path for a FileOutput.
    pub name: Option<String>,
    /// Where it came from: a FileOutput's `source` (and `taskRun` when different), a task's or
    /// agent's inputs. Empty for other kinds.
    pub inputs: Vec<ProvenanceInput>,
}

/// One input of a provenance node.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProvenanceInput {
    /// Another lineage record (visited as a node).
    Lineage {
        /// The input (or field) name.
        name: Option<String>,
        /// The record's LID.
        lid: Lid,
    },
    /// A file without lineage record, e.g. a pipeline input.
    External {
        /// The input name.
        name: Option<String>,
        /// The file path or URI, as recorded.
        path: String,
        /// Its recorded checksum.
        checksum: Option<Checksum>,
    },
    /// A plain value input.
    Value {
        /// The input name.
        name: Option<String>,
        /// The value.
        value: Value,
    },
}

/// Walks the lineage of the data file `file`: FileOutput → `source`/`taskRun` → TaskRun or
/// AgentRun → inputs → …, breadth-first, visiting each record once.
///
/// # Errors
/// [`LineageError::UnexpectedKind`] when `file` is not a FileOutput, [`LineageError::NotFound`]
/// when it or a referenced record is missing, and store errors.
pub async fn provenance(store: &dyn LineageStore, file: &Lid) -> Result<Provenance, LineageError> {
    let record = store.get(file).await?;
    if !matches!(record, Record::FileOutput(_)) {
        return Err(LineageError::UnexpectedKind {
            lid: file.to_string(),
            expected: "FileOutput",
            found: record.kind().to_owned(),
        });
    }
    let mut seen = BTreeSet::from([file.clone()]);
    let mut queue = VecDeque::from([(file.clone(), record)]);
    let mut nodes = Vec::new();
    while let Some((lid, record)) = queue.pop_front() {
        let node = node(lid, &record);
        for input in &node.inputs {
            if let ProvenanceInput::Lineage { lid, .. } = input
                && seen.insert(lid.clone())
            {
                queue.push_back((lid.clone(), store.get(lid).await?));
            }
        }
        nodes.push(node);
    }
    Ok(Provenance {
        target: file.clone(),
        nodes,
    })
}

fn node(lid: Lid, record: &Record) -> ProvenanceNode {
    let (name, inputs) = match record {
        Record::FileOutput(f) => {
            let mut inputs = Vec::new();
            for (field, value) in [("source", &f.source), ("taskRun", &f.task_run)] {
                let lid = value.as_deref().and_then(|s| Lid::parse(s).ok());
                if let Some(lid) = lid.filter(|l| !inputs.iter().any(|i| lineage_is(i, l))) {
                    let name = Some(field.to_owned());
                    inputs.push(ProvenanceInput::Lineage { name, lid });
                }
            }
            (f.path.clone(), inputs)
        }
        Record::TaskRun(t) => (t.name.clone(), params(t.input.as_deref())),
        Record::AgentRun(a) => (a.name.clone(), params(a.input.as_deref())),
        _ => (None, Vec::new()),
    };
    ProvenanceNode {
        lid,
        kind: record.kind().to_owned(),
        name,
        inputs,
    }
}

fn lineage_is(input: &ProvenanceInput, lid: &Lid) -> bool {
    matches!(input, ProvenanceInput::Lineage { lid: l, .. } if l == lid)
}

/// Each input value (list items one by one): a `lid://` string, a `{path, checksum}` object,
/// or a plain value.
fn params(input: Option<&[Parameter]>) -> Vec<ProvenanceInput> {
    let mut out = Vec::new();
    for p in input.unwrap_or_default() {
        let items = match &p.value {
            Some(Value::Array(items)) => items.clone(),
            Some(v) => vec![v.clone()],
            None => Vec::new(),
        };
        for value in items {
            let name = p.name.clone();
            let lid = value.as_str().and_then(|s| Lid::parse(s).ok());
            let path = value.get("path").and_then(Value::as_str).map(str::to_owned);
            out.push(match (lid, path) {
                (Some(lid), _) => ProvenanceInput::Lineage { name, lid },
                (None, Some(path)) => ProvenanceInput::External {
                    name,
                    path,
                    checksum: value
                        .get("checksum")
                        .and_then(|c| serde_json::from_value(c.clone()).ok()),
                },
                (None, None) => ProvenanceInput::Value { name, value },
            });
        }
    }
    out
}
