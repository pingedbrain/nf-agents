//! Read Nextflow data lineage.
//!
//! **Status: pre-alpha.** This crate is a placeholder; no API exists yet.

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_pre_release() {
        assert!(env!("CARGO_PKG_VERSION").starts_with("0."));
    }
}
