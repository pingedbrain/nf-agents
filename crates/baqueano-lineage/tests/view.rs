//! `view` edge cases the CLI oracle does not record.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use baqueano_lineage::object_store::memory::InMemory;
use baqueano_lineage::object_store::path::Path;
use baqueano_lineage::object_store::{ObjectStoreExt, PutPayload};
use baqueano_lineage::{Lid, LineageError, ObjectLineageStore, view};
use serde_json::json;

#[tokio::test]
async fn fragments_validate_redirect_and_navigate() {
    let memory = InMemory::new();
    let task = r#"{"version":"lineage/v1beta1","kind":"TaskRun","spec":{"name":"T","input":[
        {"type":"val","name":"a","value":"x"},{"type":"val","name":"b","value":null}]}}"#;
    let file = r#"{"version":"lineage/v1beta1","kind":"FileOutput","path":"/d/f.txt","size":3}"#;
    for (key, body) in [("aa/.data.json", task), ("aa/f.txt/.data.json", file)] {
        let payload = PutPayload::from(body.as_bytes().to_vec());
        memory
            .put(&Path::parse(key).unwrap(), payload)
            .await
            .unwrap();
    }
    let store = ObjectLineageStore::new(Arc::new(memory), Path::default());
    let view = |lid: &str| {
        let lid = Lid::parse(lid).unwrap();
        let store = &store;
        async move { view(store, &lid).await }
    };

    // A TaskRun without `#output` record has no outputs.
    assert_eq!(view("lid://aa#output").await.unwrap(), json!([]));
    assert_eq!(view("lid://aa#output.value").await.unwrap(), json!([]));
    assert_eq!(view("lid://aa#input.value").await.unwrap(), json!("x"));
    assert_eq!(view("lid://aa#type").await.unwrap(), json!("TaskRun"));
    assert_eq!(view("lid://aa#container").await.unwrap(), json!(null));
    // Flat (25.04) records come back in the `spec` envelope.
    let whole = view("lid://aa/f.txt").await.unwrap();
    assert_eq!(whole["spec"]["size"], json!(3));
    assert_eq!(view("lid://aa/f.txt#size").await.unwrap(), json!(3));
    assert_eq!(
        view("lid://aa/f.txt#path").await.unwrap(),
        json!("/d/f.txt")
    );

    let err = view("lid://aa#input.nope").await.unwrap_err();
    assert!(
        matches!(&err, LineageError::InvalidQuery { message } if message.contains("'nope'")),
        "{err}"
    );
    let err = view("lid://bb#name").await.unwrap_err();
    assert!(matches!(err, LineageError::NotFound { .. }), "{err}");
}
