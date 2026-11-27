//! Read Nextflow data lineage (`lineage/v1beta1`) records.
//!
//! **Status: pre-alpha.** Typed, tolerant parsing of lineage IDs ([`Lid`]) and records
//! ([`Record`]): every known field is optional and unknown fields are kept, so records written by
//! Nextflow 25.04 (flat envelope) through 26.10 (`spec` envelope) parse alike.

mod error;
mod lid;
mod model;
mod record;

pub use error::LineageError;
pub use lid::Lid;
pub use model::{
    AgentRun, Checksum, DataPath, Extra, FileOutput, Parameter, TaskOutput, TaskRun, Workflow,
    WorkflowOutput, WorkflowRun,
};
pub use record::Record;
