//! The crate's error type.

/// Every error returned by this crate.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LineageError {
    /// The input is not a valid `lid://` lineage ID.
    #[error("invalid lineage ID '{input}': {reason}")]
    InvalidLid {
        /// The rejected input.
        input: String,
        /// Why it was rejected.
        reason: String,
    },
    /// A `find` query is malformed; the message is the `nextflow lineage find` text.
    #[error("{message}")]
    InvalidQuery {
        /// What is wrong with the query.
        message: String,
    },
    /// The lineage record or file does not exist.
    #[error("{target} not found")]
    NotFound {
        /// The missing LID, key or path.
        target: String,
    },
    /// A record or store entry is not valid lineage JSON.
    #[error("invalid lineage data in {path}: {message}")]
    InvalidFormat {
        /// Where the data came from.
        path: String,
        /// The parser's message.
        message: String,
    },
    /// The record declares a lineage format version this crate does not read.
    #[error("unsupported lineage version '{version}' in {path} (expected lineage/v1beta1)")]
    UnsupportedVersion {
        /// Where the record came from.
        path: String,
        /// The declared version.
        version: String,
    },
    /// The record has another kind than the operation needs.
    #[error("{lid} is a {found}, expected a {expected}")]
    UnexpectedKind {
        /// The record's LID.
        lid: String,
        /// The kind the operation needs.
        expected: &'static str,
        /// The record's kind.
        found: String,
    },
    /// The checksum algorithm or mode cannot be verified.
    #[error("unsupported checksum algorithm '{algorithm}' (mode '{mode}') for {path}")]
    UnsupportedChecksum {
        /// The data file.
        path: String,
        /// The checksum algorithm.
        algorithm: String,
        /// The checksum mode.
        mode: String,
    },
    /// The operation is not supported for this input (scheme, option, ...).
    #[error("unsupported: {0}")]
    Unsupported(String),
}
