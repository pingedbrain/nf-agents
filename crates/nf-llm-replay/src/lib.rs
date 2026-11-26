//! Record/replay proxy for LLM HTTP APIs, so Nextflow agent pipelines can be tested offline.
//!
//! **Status: pre-alpha, for research and testing use only.** Requests are normalized into a
//! stable key ([`normalize`]) and answered from a reviewable JSONL cassette ([`cassette`]).

pub mod cassette;
pub mod normalize;
