//! Typed `lineage/v1beta1` record bodies.
//!
//! Every known field is optional (Nextflow versions differ in what they write; `null` and absent
//! are both `None`) and fields are declared in Nextflow's order, so [`crate::Record::to_json`]
//! prints them like the store does. Fields this crate does not know are kept in `extra`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Unknown fields of a record body, kept verbatim and re-emitted on serialization.
pub type Extra = Map<String, Value>;

/// A pipeline run.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRun {
    /// Script files, repository and commit.
    pub workflow: Option<Workflow>,
    /// Nextflow session UUID.
    pub session_id: Option<String>,
    /// Run name (e.g. `friendly_ride`).
    pub name: Option<String>,
    /// Pipeline parameters.
    pub params: Option<Vec<Parameter>>,
    /// Resolved configuration (free-form).
    pub config: Option<Value>,
    /// Run metadata such as `nextflow.version` (free-form, Nextflow ≥ 25.10).
    pub metadata: Option<Value>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The pipeline code of a [`WorkflowRun`].
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    /// Script files with their checksums.
    pub script_files: Option<Vec<DataPath>>,
    /// Source repository URL.
    pub repository: Option<String>,
    /// Source commit ID.
    pub commit_id: Option<String>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The outputs published by a run (stored under `<hash>#output`).
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowOutput {
    /// Creation time (ISO 8601).
    pub created_at: Option<String>,
    /// LID of the run.
    pub workflow_run: Option<String>,
    /// Published outputs.
    pub output: Option<Vec<Parameter>>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// One task execution.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRun {
    /// Nextflow session UUID.
    pub session_id: Option<String>,
    /// Process name.
    pub name: Option<String>,
    /// Checksum of the process code.
    pub code_checksum: Option<Checksum>,
    /// Resolved task script.
    pub script: Option<String>,
    /// `eval` outputs (free-form, Nextflow ≥ 25.10).
    pub eval: Option<Value>,
    /// Task inputs.
    pub input: Option<Vec<Parameter>>,
    /// Container image.
    pub container: Option<String>,
    /// Conda environment.
    pub conda: Option<String>,
    /// Spack environment.
    pub spack: Option<String>,
    /// Target architecture.
    pub architecture: Option<String>,
    /// Global variables used by the task (free-form).
    pub global_vars: Option<Value>,
    /// Files from the pipeline `bin/` directory.
    pub bin_entries: Option<Vec<DataPath>>,
    /// LID of the run.
    pub workflow_run: Option<String>,
    /// Module ID (Nextflow ≥ 25.10).
    pub module_id: Option<String>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The outputs of a task or agent (stored under `<hash>#output`).
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOutput {
    /// LID of the task or agent run.
    pub task_run: Option<String>,
    /// LID of the run.
    pub workflow_run: Option<String>,
    /// Creation time (ISO 8601).
    pub created_at: Option<String>,
    /// Outputs.
    pub output: Option<Vec<Parameter>>,
    /// Labels.
    pub labels: Option<Vec<String>>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A data file produced by a task or published by a run.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOutput {
    /// Path or URI of the file as Nextflow saw it.
    pub path: Option<String>,
    /// Checksum of the file.
    pub checksum: Option<Checksum>,
    /// LID of the record that produced it.
    pub source: Option<String>,
    /// LID of the run.
    pub workflow_run: Option<String>,
    /// LID of the producing task.
    pub task_run: Option<String>,
    /// Size in bytes.
    pub size: Option<u64>,
    /// Creation time (ISO 8601).
    pub created_at: Option<String>,
    /// Modification time (ISO 8601).
    pub modified_at: Option<String>,
    /// Labels.
    pub labels: Option<Vec<String>>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// One `agent` execution (Nextflow ≥ 26.10 with an agent plugin).
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    /// Nextflow session UUID.
    pub session_id: Option<String>,
    /// Agent name.
    pub name: Option<String>,
    /// Checksum of the agent code.
    pub code_checksum: Option<Checksum>,
    /// Agent runner (e.g. `langchain4j`).
    pub runner: Option<String>,
    /// Requested model.
    pub model: Option<String>,
    /// Model reported by the provider.
    pub resolved_model: Option<String>,
    /// System instruction.
    pub instruction: Option<String>,
    /// Goal.
    pub goal: Option<String>,
    /// Prompt template.
    pub prompt_template: Option<String>,
    /// Iteration limit.
    pub max_iterations: Option<i64>,
    /// Structured-output schema (free-form; a JSON string in 26.10).
    pub output_schema: Option<Value>,
    /// Tool names.
    pub tools: Option<Vec<String>>,
    /// Skill names.
    pub skills: Option<Vec<String>>,
    /// Agent inputs.
    pub input: Option<Vec<Parameter>>,
    /// Container image.
    pub container: Option<String>,
    /// LID of the run.
    pub workflow_run: Option<String>,
    /// Module ID.
    pub module_id: Option<String>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A checksum (`algorithm` is `nextflow` with `mode` `standard` in every known version).
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    /// Hex digest.
    pub value: Option<String>,
    /// Algorithm name.
    pub algorithm: Option<String>,
    /// Hashing mode.
    pub mode: Option<String>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A path with its checksum (script files, `bin/` entries, external task inputs).
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPath {
    /// Path or URI.
    pub path: Option<String>,
    /// Checksum.
    pub checksum: Option<Checksum>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A typed, named value (parameter, input or output).
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    /// Nextflow type name (`path`, `val`, `String`, ...).
    pub r#type: Option<String>,
    /// Name.
    pub name: Option<String>,
    /// Value: a string, a LID, a list of [`DataPath`]s, a map, ... (free-form).
    pub value: Option<Value>,
    /// Unknown fields.
    #[serde(flatten)]
    pub extra: Extra,
}
