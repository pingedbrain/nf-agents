//! Read Nextflow data lineage (`lineage/v1beta1`) records.
//!
//! **Status: pre-alpha.** Typed, tolerant parsing of lineage IDs ([`Lid`]) and records
//! ([`Record`]): every known field is optional and unknown fields are kept, so records written by
//! Nextflow 25.04 (flat envelope) through 26.10 (`spec` envelope) parse alike. [`FindQuery`]
//! reproduces `nextflow lineage find`; [`ObjectLineageStore`] reads a `.lineage` directory from
//! local disk or object storage (S3, GCS, Azure) behind the [`LineageStore`] trait. [`check`]
//! verifies a data file against its recorded checksum; [`diff`] compares records semantically.
//! Read-only.

mod check;
mod diff;
mod error;
mod lid;
mod model;
mod query;
mod record;
mod store;

pub use check::{CheckOptions, CheckReport, CheckStatus, check, nextflow_standard_checksum};
pub use diff::{Change, SemanticDiff, diff, diff_records};
pub use error::LineageError;
pub use lid::Lid;
pub use model::{
    AgentRun, Checksum, DataPath, Extra, FileOutput, Parameter, TaskOutput, TaskRun, Workflow,
    WorkflowOutput, WorkflowRun,
};
pub use query::FindQuery;
pub use record::Record;
pub use store::{LineageStore, ObjectLineageStore, RunSummary};

/// The `object_store` crate, re-exported for [`ObjectLineageStore::new`].
pub use object_store;
