//! The MCP server: one read-only tool per `nf-lineage` operation.

use std::sync::Arc;

use nf_lineage::{FindQuery, Lid, LineageError, LineageStore, Record, RunSummary};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::respond::{ToolError, finish_json};

/// Operator settings of a [`LineageServer`]; no tool parameter can change them.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The store location shown to agents in the server instructions.
    pub store_label: String,
    /// Upper bound of a tool result's text, in bytes (results above it are truncated).
    pub max_response_bytes: usize,
}

/// The MCP server over a lineage store. Read-only: no tool writes, deletes or creates anything.
#[derive(Clone)]
pub struct LineageServer {
    store: Arc<dyn LineageStore>,
    settings: Arc<Settings>,
    tool_router: ToolRouter<Self>,
}

/// Parameters of `list_runs`.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListRunsArgs {
    /// Only runs started on or after this date (`YYYY-MM-DD`, UTC as recorded).
    pub since: Option<String>,
    /// Only runs started on or before this date (`YYYY-MM-DD`, UTC as recorded).
    pub until: Option<String>,
    /// Case-insensitive part of the pipeline name (repository, project or main script).
    pub pipeline: Option<String>,
}

/// Parameters of the tools that take one lineage ID.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LidArgs {
    /// Lineage ID: `lid://<hash>[/<path>][#<field>]`, e.g. `lid://<hash>#params`.
    pub lid: String,
}

/// Parameters of `find`; at least one filter is required and all of them must match.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindArgs {
    /// Record kind: WorkflowRun, TaskRun, FileOutput, TaskOutput, WorkflowOutput or AgentRun.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Labels the record must all carry (FileOutput labels, e.g. `summary`).
    #[serde(default)]
    pub labels: Vec<String>,
    /// Record name (task or run name).
    pub name: Option<String>,
    /// More `key=value` terms in `nextflow lineage find` syntax (URL-encoded values, dotted
    /// keys), e.g. `workflowRun=lid://<hash>` or `params.greeting=hello`.
    #[serde(default)]
    pub terms: Vec<String>,
}

impl LineageServer {
    /// A server over `store` with the operator's `settings`.
    pub fn new(store: Arc<dyn LineageStore>, settings: Settings) -> Self {
        Self {
            store,
            settings: Arc::new(settings),
            tool_router: Self::tool_router(),
        }
    }

    fn cap(&self) -> usize {
        self.settings.max_response_bytes
    }

    async fn search(&self, args: FindArgs) -> Result<Vec<Lid>, ToolError> {
        // Typed fields are taken verbatim; only `terms` use the CLI's URL-decoding.
        let mut pairs: Vec<(String, String)> =
            args.kind.map(|k| ("type".into(), k)).into_iter().collect();
        pairs.extend(args.labels.into_iter().map(|l| ("labels".into(), l)));
        pairs.extend(args.name.map(|n| ("name".into(), n)));
        for (key, values) in FindQuery::parse_args(&args.terms)?.terms() {
            pairs.extend(values.iter().map(|v| (key.clone(), v.clone())));
        }
        if pairs.is_empty() {
            return Err(ToolError::Argument(
                "find needs at least one filter: type, labels, name or terms".into(),
            ));
        }
        Ok(self.store.find(&FindQuery::new(pairs)?).await?)
    }

    async fn runs(&self, args: ListRunsArgs) -> Result<Vec<RunRow>, ToolError> {
        let since = args.since.as_deref().map(|d| parse_day("since", d));
        let until = args.until.as_deref().map(|d| parse_day("until", d));
        let (since, until) = (since.transpose()?, until.transpose()?);
        let needle = args.pipeline.map(|p| p.to_lowercase());
        let mut rows = Vec::new();
        for run in self.store.list_runs().await?.into_iter().rev() {
            let day = run.timestamp.get(..10).unwrap_or_default();
            if since.is_some_and(|s| day < s) || until.is_some_and(|u| day > u) {
                continue;
            }
            // ponytail: one record read per run (sequential); parallelize if S3 latency hurts.
            let pipeline = match self.store.get(&run.lid).await {
                Ok(record) => pipeline_of(&record),
                Err(LineageError::NotFound { .. }) => None,
                Err(e) => return Err(e.into()),
            };
            let wanted = needle.as_ref().is_none_or(|n| {
                pipeline
                    .as_ref()
                    .is_some_and(|p| p.to_lowercase().contains(n))
            });
            if wanted {
                rows.push(RunRow { run, pipeline });
            }
        }
        Ok(rows)
    }
}

/// One `list_runs` row: `{timestamp, run_name, session_id, lid, pipeline}`.
#[derive(Serialize)]
struct RunRow {
    #[serde(flatten)]
    run: RunSummary,
    pipeline: Option<String>,
}

/// The pipeline a WorkflowRun ran: its repository, else the project name (Nextflow >= 26.04),
/// else the main script's file name.
fn pipeline_of(record: &Record) -> Option<String> {
    let spec = record.spec();
    let script = spec
        .pointer("/workflow/scriptFiles/0/path")
        .and_then(Value::as_str)
        .and_then(|p| p.rsplit('/').next());
    spec.pointer("/workflow/repository")
        .and_then(Value::as_str)
        .or_else(|| {
            spec.pointer("/metadata/projectName")
                .and_then(Value::as_str)
        })
        .or(script)
        .map(str::to_owned)
}

/// `value` when it is a `YYYY-MM-DD` date (compared as text with the history timestamps).
fn parse_day<'a>(name: &str, value: &'a str) -> Result<&'a str, ToolError> {
    let parts: Vec<&str> = value.split('-').collect();
    let number = |s: &str, len: usize, max: u32| {
        s.len() == len
            && s.bytes().all(|b| b.is_ascii_digit())
            && s.parse::<u32>().is_ok_and(|n| (1..=max).contains(&n))
    };
    match parts.as_slice() {
        [y, m, d] if number(y, 4, 9999) && number(m, 2, 12) && number(d, 2, 31) => Ok(value),
        _ => Err(ToolError::Argument(format!(
            "{name} '{value}' is not a date in YYYY-MM-DD form"
        ))),
    }
}

#[tool_router]
impl LineageServer {
    #[tool(
        description = "List the pipeline runs recorded in the lineage store, newest first. Use it first to get run LIDs. Optional filters: since/until (YYYY-MM-DD) and pipeline (name part). Returns a JSON array of {timestamp, run_name, session_id, lid, pipeline}, e.g. [{\"run_name\":\"friendly_ride\",\"lid\":\"lid://f93f...\",...}].",
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn list_runs(
        &self,
        Parameters(args): Parameters<ListRunsArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let hint = "use since/until dates or a pipeline filter";
        finish_json(self.runs(args).await, self.cap(), hint)
    }

    #[tool(
        description = "Show one lineage record (WorkflowRun, TaskRun, FileOutput, ...) as JSON {version, kind, spec}, or one field of it via a #fragment, e.g. lid://<hash>#params or lid://<hash>#output. Get LIDs from list_runs or find.",
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn view(
        &self,
        Parameters(args): Parameters<LidArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = match Lid::parse(&args.lid) {
            Ok(lid) => nf_lineage::view(&*self.store, &lid).await,
            Err(e) => Err(e),
        };
        let hint = "view one field with a #fragment, e.g. lid://<hash>#params";
        finish_json(result.map_err(ToolError::from), self.cap(), hint)
    }

    #[tool(
        description = "Find lineage records by type, labels, name or key=value terms (dotted keys allowed, e.g. params.greeting=hello); all filters must match. Returns a sorted JSON array of LIDs, e.g. {\"type\":\"FileOutput\",\"labels\":[\"summary\"]} -> [\"lid://<hash>/summary.txt\"].",
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn find(
        &self,
        Parameters(args): Parameters<FindArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let hint = "add type, labels, name or key=value terms";
        finish_json(self.search(args).await, self.cap(), hint)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LineageServer {
    fn get_info(&self) -> ServerConfig {
        let instructions = format!(
            "Query the Nextflow data lineage store at {store}. This server is read-only: \
             nothing is ever modified. Tools: list_runs (start here: runs newest first, with their LIDs), find \
             (records by type/labels/name/key=value terms), view (one record, or one field with \
             #fragment). Lineage IDs look like lid://<hash>[/<path>][#<field>]. Results above \
             {cap} bytes are truncated with a final [truncated] note: narrow the request.",
            store = self.settings.store_label,
            cap = self.cap(),
        );
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "nf-lineage-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(instructions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_prefers_repository_then_project_then_script() {
        let run = |spec: &str| {
            let json =
                format!(r#"{{"version":"lineage/v1beta1","kind":"WorkflowRun","spec":{spec}}}"#);
            pipeline_of(&Record::parse(json.as_bytes(), "test").unwrap())
        };
        let script = r#"{"scriptFiles":[{"path":"file:///p/main.nf"}]"#;
        let repo = format!(
            r#"{{"workflow":{script},"repository":"https://github.com/nf-core/sarek"}},"metadata":{{"projectName":"nf-core/sarek"}}}}"#
        );
        assert_eq!(
            run(&repo).as_deref(),
            Some("https://github.com/nf-core/sarek")
        );
        let project = format!(r#"{{"workflow":{script}}},"metadata":{{"projectName":"demo"}}}}"#);
        assert_eq!(run(&project).as_deref(), Some("demo"));
        assert_eq!(
            run(&format!(r#"{{"workflow":{script}}}}}"#)).as_deref(),
            Some("main.nf")
        );
        assert_eq!(run("{}"), None);
    }

    #[test]
    fn dates_must_be_iso_days() {
        assert_eq!(parse_day("since", "2026-11-26").unwrap(), "2026-11-26");
        for bad in [
            "yesterday",
            "2026-1-26",
            "2026-13-01",
            "2026-11-32",
            "2026/11/26",
            "",
        ] {
            let Err(ToolError::Argument(message)) = parse_day("since", bad) else {
                panic!("{bad} accepted");
            };
            assert!(
                message.contains("since") && message.contains(bad),
                "{message}"
            );
        }
    }
}
