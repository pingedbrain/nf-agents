//! `view`: a record or a property of it, like `nextflow lineage view`.

use serde_json::{Value, json};

use crate::query::{navigate, validate_property};
use crate::{Lid, LineageError, LineageStore, Record};

/// The record `lid` names as canonical `{"version", "kind", "spec"}` JSON, or with a fragment
/// (`lid://<hash>#params`) the property it names: every dot segment must be a model property;
/// `output` on a WorkflowRun, TaskRun or AgentRun reads the `#output` record (`[]` when there is
/// none); `type` is the record kind; a path that leads nowhere is `null`.
///
/// # Errors
/// [`LineageError::NotFound`] for a missing record, [`LineageError::InvalidQuery`] for an
/// unknown property, and store errors.
pub async fn view(store: &dyn LineageStore, lid: &Lid) -> Result<Value, LineageError> {
    let base = match lid.path() {
        Some(path) => format!("lid://{}/{path}", lid.hash()),
        None => format!("lid://{}", lid.hash()),
    };
    let base = Lid::parse(&base)?;
    let record = store.get(&base).await?;
    let Some(fragment) = lid.fragment() else {
        return Ok(record.to_json());
    };
    let path: Vec<&str> = fragment.split('.').collect();
    path.iter()
        .try_for_each(|segment| validate_property(segment))?;
    let is_run = matches!(
        record,
        Record::WorkflowRun(_) | Record::TaskRun(_) | Record::AgentRun(_)
    );
    if is_run && path.first() == Some(&"output") {
        // Nextflow keeps run outputs under `<key>#output`; only path-less keys have one.
        let outputs = match lid.path() {
            None => store.get(&Lid::parse(&format!("{base}#output"))?).await,
            Some(_) => Err(LineageError::NotFound {
                target: lid.to_string(),
            }),
        };
        return match outputs {
            Ok(outputs) => Ok(navigate(&outputs.spec(), &path)),
            Err(LineageError::NotFound { .. }) => Ok(json!([])),
            Err(e) => Err(e),
        };
    }
    if fragment == "type" {
        return Ok(record.kind().into());
    }
    Ok(navigate(&record.spec(), &path))
}
