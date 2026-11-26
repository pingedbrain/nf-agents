//! Every tool called through MCP (rmcp client and server over an in-memory duplex pipe).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{call, call_json, core_versions, ids, oracle_lines, start, try_call};
use serde_json::{Value, json};

const CAP: usize = 32_768;
const LATEST: &str = "26.10.7";

#[tokio::test]
async fn server_info_and_tool_catalogue() {
    let client = start(LATEST, CAP).await;
    let info = client.peer_info().unwrap();
    let server = info.server_info.as_ref().unwrap();
    assert_eq!(server.name, "nf-lineage-mcp");
    assert_eq!(server.version, env!("CARGO_PKG_VERSION"));
    assert!(info.capabilities.tools.is_some());
    let instructions = info.instructions.as_deref().unwrap();
    assert!(instructions.contains("read-only"), "{instructions}");
    assert!(instructions.contains(".lineage"), "{instructions}");
    assert!(instructions.contains("lid://"), "{instructions}");

    let tools = client.list_all_tools().await.unwrap();
    let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    names.sort_unstable();
    assert_eq!(names, ["find", "list_runs", "view"]);
    for tool in &tools {
        assert!(tool.description.as_ref().is_some_and(|d| d.len() > 40));
        let a = tool.annotations.as_ref().unwrap();
        assert_eq!(a.read_only_hint, Some(true), "{}", tool.name);
        assert_eq!(a.idempotent_hint, Some(true));
        assert_eq!(a.destructive_hint, Some(false));
        assert_eq!(a.open_world_hint, Some(false));
        let schema = Value::Object((*tool.input_schema).clone());
        assert_eq!(schema["additionalProperties"], false, "{}", tool.name);
        for (name, property) in schema["properties"].as_object().unwrap() {
            assert!(property["description"].is_string(), "{}.{name}", tool.name);
        }
    }
    client.cancel().await.unwrap();
}

/// `oracle/list.out` rows as `[timestamp, run_name, session_id, lid]`, newest first.
fn oracle_runs(version: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = oracle_lines(version, "list")
        .iter()
        .skip(1)
        .map(|line| line.split('\t').map(|f| f.trim().to_owned()).collect())
        .collect();
    rows.reverse();
    rows
}

#[tokio::test]
async fn list_runs_every_core_version() {
    for version in core_versions() {
        let client = start(&version, CAP).await;
        let runs = call_json(&client, "list_runs", json!({})).await;
        let got: Vec<Vec<String>> = runs
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                ["timestamp", "run_name", "session_id", "lid"]
                    .iter()
                    .map(|k| r[k].as_str().unwrap().to_owned())
                    .collect()
            })
            .collect();
        assert_eq!(got.len(), 2, "{version}");
        assert_eq!(got, oracle_runs(&version), "{version}");
        assert!(runs[0]["pipeline"].is_string(), "{version}: {runs}");
        client.cancel().await.unwrap();
    }
}

#[tokio::test]
async fn list_runs_filters() {
    for version in core_versions() {
        let client = start(&version, CAP).await;
        let day = oracle_runs(&version)[0][0][..10].to_owned();
        let count = |args: Value| {
            let client = &client;
            async move {
                call_json(client, "list_runs", args)
                    .await
                    .as_array()
                    .unwrap()
                    .len()
            }
        };
        assert_eq!(count(json!({"since": "2999-01-01"})).await, 0, "{version}");
        assert_eq!(
            count(json!({"since": day, "until": day})).await,
            2,
            "{version}"
        );
        assert_eq!(count(json!({"until": "2000-01-01"})).await, 0, "{version}");
        assert_eq!(count(json!({"pipeline": "MAIN"})).await, 2, "{version}");
        assert_eq!(count(json!({"pipeline": "sarek"})).await, 0, "{version}");
        client.cancel().await.unwrap();
    }
}

#[tokio::test]
async fn view_record_and_fragment() {
    let client = start(LATEST, CAP).await;
    let run1 = &ids(LATEST)["RUN1"];
    let record = call_json(&client, "view", json!({"lid": run1})).await;
    assert_eq!(record["kind"], "WorkflowRun");
    let params = call_json(&client, "view", json!({"lid": format!("{run1}#params")})).await;
    assert!(params.to_string().contains("greeting"), "{params}");
    assert!(params.is_array(), "{params}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn find_matches_oracle() {
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    let as_strings = |v: Value| -> Vec<String> {
        let items = v.as_array().unwrap().iter();
        items.map(|s| s.as_str().unwrap().to_owned()).collect()
    };
    for version in core_versions() {
        let client = start(&version, CAP).await;
        let run1 = &ids(&version)["RUN1"];
        let cases = [
            (
                json!({"type": "FileOutput", "labels": ["summary"]}),
                "find-fileoutput-label",
            ),
            (
                json!({"terms": ["type=TaskRun", format!("workflowRun={run1}")]}),
                "find-taskrun",
            ),
        ];
        for (args, oracle) in cases {
            let got = as_strings(call_json(&client, "find", args).await);
            assert_eq!(got, sorted(got.clone()), "{version} {oracle}: not sorted");
            // Nextflow 25.04 prints a JSON array, later versions one LID per line.
            let lines = oracle_lines(&version, oracle);
            let expected = match serde_json::from_str(&lines.concat()) {
                Ok(array) => as_strings(array),
                Err(_) => lines,
            };
            assert_eq!(got, sorted(expected), "{version} {oracle}");
        }
        client.cancel().await.unwrap();
    }
    let client = start("26.10.7-agent", CAP).await;
    let agents = as_strings(call_json(&client, "find", json!({"type": "AgentRun"})).await);
    assert!(!agents.is_empty());
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn errors_are_tool_results_with_hints() {
    let client = start(LATEST, CAP).await;
    let missing = format!("lid://{}", "0".repeat(32));
    let cases = [
        ("view", json!({"lid": "foo"}), "invalid_lid", "foo"),
        (
            "view",
            json!({"lid": missing}),
            "not_found",
            missing.as_str(),
        ),
        (
            "find",
            json!({"terms": ["foo=bar"]}),
            "invalid_query",
            "foo",
        ),
        ("find", json!({"terms": ["noeq"]}), "invalid_query", "noeq"),
        ("find", json!({}), "invalid_argument", "filter"),
        (
            "list_runs",
            json!({"since": "yesterday"}),
            "invalid_argument",
            "yesterday",
        ),
    ];
    for (tool, args, class, named) in cases {
        let (is_error, texts) = call(&client, tool, args).await;
        assert!(is_error, "{tool}: {texts:?}");
        let text = &texts[0];
        assert!(text.starts_with(&format!("{class}: ")), "{text}");
        assert!(text.contains(named) && text.contains(" Hint: "), "{text}");
    }
    // Unknown fields are rejected by rmcp's argument parsing, still as a tool error.
    let (is_error, texts) = call(&client, "view", json!({"lidd": "x"})).await;
    assert!(is_error && texts[0].contains("unknown field"), "{texts:?}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn unknown_tool_is_protocol_error() {
    let client = start(LATEST, CAP).await;
    let err = try_call(&client, "render", json!({})).await.unwrap_err();
    let rmcp::service::ServiceError::McpError(data) = err else {
        panic!("not a JSON-RPC error: {err:?}");
    };
    assert_eq!(data.code.0, -32602);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn truncation_notice() {
    // The fixture's FileOutput list is ~530 bytes, so a cap below it forces item-wise truncation.
    let cap = 512;
    let client = start(LATEST, cap).await;
    let total = |texts: &[String]| texts.iter().map(String::len).sum::<usize>();

    let (is_error, texts) = call(&client, "find", json!({"type": "FileOutput"})).await;
    assert!(!is_error && texts.len() == 2, "{texts:?}");
    let shown: Vec<String> = serde_json::from_str(&texts[0]).unwrap();
    assert!(!shown.is_empty());
    assert!(texts[1].starts_with("[truncated]"), "{}", texts[1]);
    assert!(
        texts[1].contains(&format!("{} of 10 items", shown.len())),
        "{}",
        texts[1]
    );
    assert!(total(&texts) <= cap);

    let run1 = &ids(LATEST)["RUN1"];
    let (is_error, texts) = call(&client, "view", json!({"lid": run1})).await;
    assert!(!is_error && texts.len() == 2, "{texts:?}");
    assert!(texts[0].starts_with("{\"version\""));
    assert!(texts[1].starts_with("[truncated]") && texts[1].contains("#fragment"));
    assert!(total(&texts) <= cap);
    client.cancel().await.unwrap();
}
