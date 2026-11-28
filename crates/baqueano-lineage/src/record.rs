//! The `lineage/v1beta1` record envelope.

use serde::de::DeserializeOwned;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value, json};

use crate::LineageError;
use crate::model::{AgentRun, FileOutput, TaskOutput, TaskRun, WorkflowOutput, WorkflowRun};

/// A lineage record, typed by its `kind`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    /// A pipeline run.
    WorkflowRun(WorkflowRun),
    /// The outputs published by a run.
    WorkflowOutput(WorkflowOutput),
    /// A task execution.
    TaskRun(TaskRun),
    /// The outputs of a task or agent.
    TaskOutput(TaskOutput),
    /// A data file.
    FileOutput(FileOutput),
    /// An agent execution.
    AgentRun(AgentRun),
    /// Any other kind: the whole record JSON, verbatim.
    Unknown(Value),
}

impl Record {
    /// The only lineage format version this crate reads.
    pub const VERSION: &str = "lineage/v1beta1";

    /// Parses a record from its JSON bytes. `origin` (a path or key) is only used in errors.
    ///
    /// Accepts the flat envelope (Nextflow 25.04: fields at the root) and the `spec` envelope.
    ///
    /// # Errors
    /// [`LineageError::UnsupportedVersion`] for another `version` string, else
    /// [`LineageError::InvalidFormat`] for malformed JSON, a missing `version`/`kind` or a known
    /// field of the wrong type.
    pub fn parse(bytes: &[u8], origin: &str) -> Result<Self, LineageError> {
        let invalid = |message: String| LineageError::InvalidFormat {
            path: origin.to_owned(),
            message,
        };
        let mut root: Map<String, Value> =
            serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))?;
        match root.get("version") {
            Some(Value::String(v)) if v == Self::VERSION => {}
            Some(Value::String(v)) => {
                return Err(LineageError::UnsupportedVersion {
                    path: origin.to_owned(),
                    version: v.clone(),
                });
            }
            _ => return Err(invalid("missing or non-string 'version'".to_owned())),
        }
        let Some(Value::String(kind)) = root.get("kind").cloned() else {
            return Err(invalid("missing or non-string 'kind'".to_owned()));
        };
        if !KNOWN.contains(&kind.as_str()) {
            return Ok(Self::Unknown(Value::Object(root)));
        }
        let fields = match root.remove("spec") {
            Some(spec) => spec,
            None => Value::Object(body(root)),
        };
        fn typed<T: DeserializeOwned>(v: Value, origin: &str) -> Result<T, LineageError> {
            serde_json::from_value(v).map_err(|e| LineageError::InvalidFormat {
                path: origin.to_owned(),
                message: e.to_string(),
            })
        }
        Ok(match kind.as_str() {
            "WorkflowRun" => Self::WorkflowRun(typed(fields, origin)?),
            "WorkflowOutput" => Self::WorkflowOutput(typed(fields, origin)?),
            "TaskRun" => Self::TaskRun(typed(fields, origin)?),
            "TaskOutput" => Self::TaskOutput(typed(fields, origin)?),
            "FileOutput" => Self::FileOutput(typed(fields, origin)?),
            _ => Self::AgentRun(typed(fields, origin)?),
        })
    }

    /// The record kind, e.g. `TaskRun` (for [`Record::Unknown`], its `kind` string).
    pub fn kind(&self) -> &str {
        match self {
            Self::WorkflowRun(_) => "WorkflowRun",
            Self::WorkflowOutput(_) => "WorkflowOutput",
            Self::TaskRun(_) => "TaskRun",
            Self::TaskOutput(_) => "TaskOutput",
            Self::FileOutput(_) => "FileOutput",
            Self::AgentRun(_) => "AgentRun",
            Self::Unknown(raw) => raw.get("kind").and_then(Value::as_str).unwrap_or_default(),
        }
    }

    /// The record body: known fields (absent ones as `null`) followed by unknown ones.
    pub fn spec(&self) -> Value {
        // Serializing these types cannot fail (string keys only).
        match self {
            Self::WorkflowRun(r) => serde_json::to_value(r).unwrap_or_default(),
            Self::WorkflowOutput(r) => serde_json::to_value(r).unwrap_or_default(),
            Self::TaskRun(r) => serde_json::to_value(r).unwrap_or_default(),
            Self::TaskOutput(r) => serde_json::to_value(r).unwrap_or_default(),
            Self::FileOutput(r) => serde_json::to_value(r).unwrap_or_default(),
            Self::AgentRun(r) => serde_json::to_value(r).unwrap_or_default(),
            Self::Unknown(raw) => match raw.get("spec") {
                Some(spec) => spec.clone(),
                None => Value::Object(body(raw.as_object().cloned().unwrap_or_default())),
            },
        }
    }

    /// The record as `{"version", "kind", "spec"}` JSON (the 26.x envelope).
    pub fn to_json(&self) -> Value {
        json!({"version": Self::VERSION, "kind": self.kind(), "spec": self.spec()})
    }
}

impl Serialize for Record {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_json().serialize(serializer)
    }
}

const KNOWN: [&str; 6] = [
    "WorkflowRun",
    "WorkflowOutput",
    "TaskRun",
    "TaskOutput",
    "FileOutput",
    "AgentRun",
];

/// A flat record minus its envelope keys.
fn body(mut root: Map<String, Value>) -> Map<String, Value> {
    root.remove("version");
    root.remove("kind");
    root
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn task_fields() -> Value {
        json!({"sessionId": "s1", "name": "TRANSFORM", "script": "tr a-z A-Z",
            "codeChecksum": {"value": "eca6", "algorithm": "nextflow", "mode": "standard"},
            "input": [{"type": "path", "name": "raw", "value": ["lid://ab/raw.txt"]}],
            "container": null, "workflowRun": "lid://cd"})
    }

    fn parse(v: &Value) -> Result<Record, LineageError> {
        Record::parse(v.to_string().as_bytes(), "mem://x")
    }

    #[test]
    fn parses_flat_envelope() {
        let mut flat = task_fields();
        flat["version"] = json!("lineage/v1beta1");
        flat["kind"] = json!("TaskRun");
        let Record::TaskRun(task) = parse(&flat).unwrap() else {
            panic!("not a TaskRun")
        };
        assert_eq!(task.name.as_deref(), Some("TRANSFORM"));
        assert_eq!(task.code_checksum.unwrap().value.as_deref(), Some("eca6"));
        let input = task.input.unwrap();
        assert_eq!(input[0].r#type.as_deref(), Some("path"));
        assert_eq!(input[0].value, Some(json!(["lid://ab/raw.txt"])));
        assert!(task.extra.is_empty());
    }

    #[test]
    fn parses_spec_envelope() {
        let mut flat = task_fields();
        flat["version"] = json!("lineage/v1beta1");
        flat["kind"] = json!("TaskRun");
        let spec = json!({"version": "lineage/v1beta1", "kind": "TaskRun", "spec": task_fields()});
        let record = parse(&spec).unwrap();
        assert_eq!(record, parse(&flat).unwrap());
        assert_eq!(record.kind(), "TaskRun");
        assert_eq!(record.to_json()["spec"]["name"], "TRANSFORM");
        assert_eq!(record.to_json()["spec"]["conda"], Value::Null);
        let keys: Vec<_> = record
            .to_json()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys, ["version", "kind", "spec"]);
        assert_eq!(serde_json::to_value(&record).unwrap(), record.to_json());
    }

    #[test]
    fn unknown_kind_is_preserved() {
        let raw = json!({"version": "lineage/v1beta1", "kind": "DatasetRun", "spec": {"a": [1]}});
        let record = parse(&raw).unwrap();
        assert_eq!(record, Record::Unknown(raw.clone()));
        assert_eq!(record.kind(), "DatasetRun");
        assert_eq!(record.spec(), json!({"a": [1]}));
        assert_eq!(record.to_json(), raw);
        let flat = json!({"version": "lineage/v1beta1", "kind": "X", "a": 1});
        assert_eq!(parse(&flat).unwrap().spec(), json!({"a": 1}));
    }

    #[test]
    fn unknown_fields_are_preserved_and_reserialized() {
        let raw = json!({"version": "lineage/v1beta1", "kind": "FileOutput",
            "spec": {"path": "/a.txt", "size": 9, "futureField": {"x": 1}}});
        let record = parse(&raw).unwrap();
        let Record::FileOutput(file) = &record else {
            panic!("not a FileOutput")
        };
        assert_eq!(file.size, Some(9));
        assert_eq!(file.extra["futureField"], json!({"x": 1}));
        assert_eq!(record.to_json()["spec"]["futureField"], json!({"x": 1}));
    }

    #[test]
    fn unsupported_version_names_version() {
        let err = parse(&json!({"version": "lineage/v2", "kind": "TaskRun"})).unwrap_err();
        assert!(
            matches!(err, LineageError::UnsupportedVersion { .. }),
            "{err:?}"
        );
        assert!(err.to_string().contains("lineage/v2"));
        assert!(err.to_string().contains("mem://x"));
    }

    #[test]
    fn invalid_inputs_are_invalid_format() {
        let nested = format!("{}{}", "[".repeat(10_000), "]".repeat(10_000));
        let inputs = [
            String::new(),
            "[]".into(),
            "{".into(),
            r#"{"kind": "TaskRun"}"#.into(),
            r#"{"version": 1, "kind": "TaskRun"}"#.into(),
            r#"{"version": "lineage/v1beta1"}"#.into(),
            r#"{"version": "lineage/v1beta1", "kind": "FileOutput", "size": "nine"}"#.into(),
            r#"{"version": "lineage/v1beta1", "kind": "TaskRun", "spec": []}"#.into(),
            nested,
        ];
        for input in inputs {
            match Record::parse(input.as_bytes(), "mem://x") {
                Err(LineageError::InvalidFormat { path, message }) => {
                    assert_eq!(path, "mem://x");
                    assert!(!message.is_empty());
                }
                other => panic!("{:.40} gave {other:?}", input),
            }
        }
    }
}
