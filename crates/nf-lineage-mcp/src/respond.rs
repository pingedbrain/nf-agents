//! Tool results: the response size cap and the mapping of failures to `isError` results.

use nf_lineage::LineageError;
use rmcp::ErrorData;
use rmcp::model::{CallToolResult, ContentBlock};
use serde::Serialize;
use serde_json::{Map, Value};

/// Bytes kept free for the `[truncated]` notice (hints are short constants).
const NOTICE_RESERVE: usize = 256;

/// Why a tool call failed: a lineage error or a bad argument found by the server.
#[derive(Debug)]
pub(crate) enum ToolError {
    Lineage(LineageError),
    Argument(String),
}

impl From<LineageError> for ToolError {
    fn from(e: LineageError) -> Self {
        Self::Lineage(e)
    }
}

/// The result of a JSON-returning tool: the capped value, or the failure as an `isError` result.
/// Only a value that cannot be serialized (a bug) is a JSON-RPC error.
pub(crate) fn finish_json<T: Serialize>(
    result: Result<T, ToolError>,
    cap: usize,
    hint: &str,
) -> Result<CallToolResult, ErrorData> {
    match result {
        Ok(value) => serde_json::to_value(value)
            .map(|v| fit_json(&v, cap, hint))
            .map_err(|e| ErrorData::internal_error(e.to_string(), None)),
        Err(e) => Ok(tool_error(&e)),
    }
}

/// The result of a text-returning tool: the capped text, or the failure as an `isError` result.
pub(crate) fn finish_text(
    result: Result<String, ToolError>,
    cap: usize,
    hint: &str,
) -> CallToolResult {
    match result {
        Ok(text) => fit_text(text, cap, hint),
        Err(e) => tool_error(&e),
    }
}

/// Compact JSON within `cap` bytes: arrays keep their longest prefix of whole items (still valid
/// JSON); an object keeps its other keys and cuts its largest array the same way; anything else
/// is cut as text. `hint` tells the agent how to narrow the request.
pub(crate) fn fit_json(value: &Value, cap: usize, hint: &str) -> CallToolResult {
    let text = value.to_string();
    if text.len() <= cap {
        return fit_text(text, cap, hint);
    }
    match value {
        Value::Array(items) => fit_array(items, cap, hint),
        Value::Object(map) => {
            fit_object(map, cap, hint).unwrap_or_else(|| fit_text(text, cap, hint))
        }
        _ => fit_text(text, cap, hint),
    }
}

/// An object over `cap` with its largest array cut to a prefix of whole items. `None` when the
/// object has no array field, or when its other fields alone exceed the cap.
fn fit_object(map: &Map<String, Value>, cap: usize, hint: &str) -> Option<CallToolResult> {
    let (key, items) = map
        .iter()
        .filter_map(|(k, v)| Some((k, v.as_array()?)))
        .max_by_key(|(_, items)| items.iter().map(|v| v.to_string().len()).sum::<usize>())?;
    let mut skeleton = map.clone();
    skeleton.insert(key.clone(), Value::Array(Vec::new()));
    // The object with an empty array: everything else is kept whatever the budget.
    let base = Value::Object(skeleton.clone()).to_string().len();
    let budget = cap.saturating_sub(NOTICE_RESERVE).checked_sub(base)?;
    let mut kept = Vec::new();
    let mut used = 0;
    for item in items {
        // `,` before every item but the first.
        let added = item.to_string().len() + usize::from(!kept.is_empty());
        if used + added > budget {
            break;
        }
        used += added;
        kept.push(item.clone());
    }
    let shown = kept.len();
    skeleton.insert(key.clone(), Value::Array(kept));
    let notice = format!(
        "[truncated] showing {shown} of {} items in \"{key}\" ({cap}-byte response limit). Narrow the request: {hint}.",
        items.len()
    );
    Some(CallToolResult::success(vec![
        ContentBlock::text(Value::Object(skeleton).to_string()),
        ContentBlock::text(notice),
    ]))
}

/// An array over `cap`: its longest prefix of whole items plus a notice.
fn fit_array(items: &[Value], cap: usize, hint: &str) -> CallToolResult {
    let budget = cap.saturating_sub(NOTICE_RESERVE);
    let mut shown = String::from("[");
    let mut count = 0;
    for item in items {
        let item = item.to_string();
        // `,` before the item and the closing `]`.
        if shown.len() + item.len() + 2 > budget {
            break;
        }
        if count > 0 {
            shown.push(',');
        }
        shown.push_str(&item);
        count += 1;
    }
    shown.push(']');
    let notice = format!(
        "[truncated] showing {count} of {} items ({cap}-byte response limit). Narrow the request: {hint}.",
        items.len()
    );
    CallToolResult::success(vec![ContentBlock::text(shown), ContentBlock::text(notice)])
}

/// `text` within `cap` bytes, cut at a char boundary with a `[truncated]` notice when too long.
pub(crate) fn fit_text(mut text: String, cap: usize, hint: &str) -> CallToolResult {
    if text.len() <= cap {
        return CallToolResult::success(vec![ContentBlock::text(text)]);
    }
    let total = text.len();
    let mut end = cap.saturating_sub(NOTICE_RESERVE);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    let notice = format!(
        "[truncated] showing {end} of {total} bytes ({cap}-byte response limit). Narrow the request: {hint}."
    );
    CallToolResult::success(vec![ContentBlock::text(text), ContentBlock::text(notice)])
}

/// A failure as an `isError` tool result: `<class>: <message> Hint: <next step>`.
pub(crate) fn tool_error(error: &ToolError) -> CallToolResult {
    let (class, hint) = match error {
        ToolError::Argument(_) => (
            "invalid_argument",
            "fix the argument as described in the tool's input schema and call again.",
        ),
        ToolError::Lineage(e) => match e {
            LineageError::InvalidLid { .. } => (
                "invalid_lid",
                "use a lineage ID like lid://<hash>[/<path>][#<field>]; get run LIDs from list_runs and record LIDs from find.",
            ),
            LineageError::InvalidQuery { .. } => (
                "invalid_query",
                "keys must be lineage model properties (type, name, labels, workflowRun, taskRun, params.<name>, ...), written key=value.",
            ),
            LineageError::NotFound { .. } => (
                "not_found",
                "get existing LIDs from list_runs (runs) or find (any record).",
            ),
            LineageError::UnexpectedKind { .. } => (
                "unexpected_kind",
                "this tool needs a FileOutput LID; get one with find type=FileOutput, terms workflowRun=<run LID>.",
            ),
            LineageError::InvalidFormat { .. } | LineageError::UnsupportedVersion { .. } => (
                "invalid_record",
                "the store holds a record this server cannot read; try another record.",
            ),
            LineageError::UnsupportedChecksum { .. } | LineageError::Unsupported(_) => (
                "unsupported",
                "this operation is not supported for this record or store; try another tool.",
            ),
            LineageError::Storage(_) | LineageError::Io { .. } => (
                "storage",
                "the lineage store could not be read (unreachable, permissions or credentials); retry later or ask the operator to check the store.",
            ),
            _ => (
                "error",
                "unexpected failure; retry, or report it with the message.",
            ),
        },
    };
    let message = match error {
        ToolError::Argument(m) => m.clone(),
        ToolError::Lineage(e) => e.to_string(),
    };
    tracing::warn!(class, %message, "tool call failed");
    CallToolResult::error(vec![ContentBlock::text(format!(
        "{class}: {message} Hint: {hint}"
    ))])
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn texts(result: &CallToolResult) -> Vec<String> {
        result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.clone()))
            .collect()
    }

    fn total(texts: &[String]) -> usize {
        texts.iter().map(String::len).sum()
    }

    #[test]
    fn fit_rules() {
        // Small payload: untouched, one block.
        let small = fit_json(&json!({"a": 1}), 1024, "unused");
        assert_eq!(texts(&small), [r#"{"a":1}"#]);
        assert_ne!(small.is_error, Some(true));

        // Large array: longest prefix of whole items (valid JSON) plus a notice.
        let big: Value = (0..500).map(|i| json!(format!("lid://{i:032}"))).collect();
        let t = texts(&fit_json(&big, 1024, "add filters"));
        assert_eq!(t.len(), 2);
        let shown: Vec<Value> = serde_json::from_str(&t[0]).unwrap();
        assert!(!shown.is_empty() && shown.len() < 500);
        assert_eq!(shown[..], big.as_array().unwrap()[..shown.len()]);
        assert!(t[1].starts_with("[truncated]"), "{}", t[1]);
        assert!(t[1].contains(&format!("{} of 500 items", shown.len())));
        assert!(t[1].contains("add filters"));
        assert!(total(&t) <= 1024);

        // An array whose first item alone is too big: empty array plus the notice.
        let huge = json!(["x".repeat(2000)]);
        let t = texts(&fit_json(&huge, 1024, "h"));
        assert_eq!(t[0], "[]");
        assert!(t[1].contains("0 of 1 items"));

        // Non-array JSON and text: cut at a char boundary, never splitting a code point.
        for cap in [1024, 1025] {
            let text = "é".repeat(1000);
            let t = texts(&fit_text(text.clone(), cap, "use a #fragment"));
            assert_eq!(t.len(), 2);
            assert!(text.starts_with(&t[0]) && !t[0].is_empty() && t[0].len() < text.len());
            assert!(t[1].starts_with("[truncated]"));
            assert!(t[1].contains(&format!("{} of 2000 bytes", t[0].len())));
            assert!(total(&t) <= cap);
        }
        let object = json!({"text": "é".repeat(1000)});
        assert_eq!(texts(&fit_json(&object, 1024, "h")).len(), 2);
    }

    #[test]
    fn object_keeps_other_keys_and_cuts_its_largest_array() {
        let nodes: Value = (0..500).map(|i| json!(format!("lid://{i:032}"))).collect();
        let object = json!({"target": "x", "nodes": nodes, "edges": [1]});
        let t = texts(&fit_json(&object, 1024, "narrow the depth"));
        assert_eq!(t.len(), 2);
        let shown: Value = serde_json::from_str(&t[0]).unwrap();
        assert_eq!(shown["target"], "x");
        assert_eq!(shown["edges"], json!([1]));
        let kept = shown["nodes"].as_array().unwrap();
        assert!(!kept.is_empty() && kept.len() < 500);
        assert_eq!(kept[..], nodes.as_array().unwrap()[..kept.len()]);
        assert!(t[1].starts_with("[truncated]"), "{}", t[1]);
        assert!(t[1].contains(&format!("{} of 500 items in \"nodes\"", kept.len())));
        assert!(t[1].contains("narrow the depth"));
        assert!(total(&t) <= 1024);

        // Other fields too big for the cap: the text cut applies.
        let heavy = json!({"target": "x".repeat(2000), "nodes": [1, 2]});
        assert_eq!(texts(&fit_json(&heavy, 1024, "h")).len(), 2);
    }

    #[test]
    fn every_error_variant_has_class_and_hint() {
        use LineageError as E;
        let s = |v: &str| v.to_owned();
        let store = || nf_lineage::object_store::Error::Generic {
            store: "S3",
            source: "down".into(),
        };
        #[rustfmt::skip]
        let cases: Vec<(ToolError, &str)> = vec![
            (E::InvalidLid { input: s("foo"), reason: s("r") }.into(), "invalid_lid"),
            (E::InvalidQuery { message: s("bad") }.into(), "invalid_query"),
            (E::NotFound { target: s("lid://x") }.into(), "not_found"),
            (E::InvalidFormat { path: s("p"), message: s("m") }.into(), "invalid_record"),
            (E::UnsupportedVersion { path: s("p"), version: s("v9") }.into(), "invalid_record"),
            (E::UnexpectedKind { lid: s("x"), expected: "FileOutput", found: s("TaskRun") }.into(), "unexpected_kind"),
            (E::UnsupportedChecksum { path: s("p"), algorithm: s("md5"), mode: s("m") }.into(), "unsupported"),
            (E::Unsupported(s("x")).into(), "unsupported"),
            (E::Storage(store()).into(), "storage"),
            (E::Io { path: s("p"), source: std::io::Error::other("boom") }.into(), "storage"),
            (ToolError::Argument(s("bad date")), "invalid_argument"),
        ];
        for (error, class) in cases {
            let result = tool_error(&error);
            assert_eq!(result.is_error, Some(true));
            let t = texts(&result);
            assert_eq!(t.len(), 1);
            let (head, hint) = t[0].split_once(" Hint: ").unwrap();
            assert!(head.starts_with(&format!("{class}: ")), "{head}");
            assert!(hint.len() > 10, "{class}: {hint}");
        }
    }
}
