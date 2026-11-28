//! `nextflow lineage find` queries.

use serde_json::Value;

use crate::{LineageError, Record};

/// Every property name of the lineage model (Nextflow's `LinPropertyValidator`, incl. `class`).
const PROPERTIES: &str = "workflow sessionId name params config metadata createdAt workflowRun \
    output codeChecksum script eval input container conda spack architecture globalVars binEntries \
    moduleId taskRun labels path checksum source size modifiedAt runner model resolvedModel \
    instruction goal promptTemplate maxIterations outputSchema tools skills value algorithm mode \
    type scriptFiles repository commitId class";

/// A `nextflow lineage find` query: `key=value` terms that must all match a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindQuery {
    terms: Vec<(String, Vec<String>)>,
}

impl FindQuery {
    /// Builds a query from already-decoded `(key, value)` pairs. `label` is an alias of
    /// `labels`; repeated keys accumulate their values.
    ///
    /// # Errors
    /// [`LineageError::InvalidQuery`] when a dot segment of a key is not a model property.
    pub fn new<K: Into<String>, V: Into<String>>(
        pairs: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self, LineageError> {
        let mut terms: Vec<(String, Vec<String>)> = Vec::new();
        for (key, value) in pairs {
            let mut key = key.into();
            if key == "label" {
                key = "labels".to_owned();
            }
            match terms.iter_mut().find(|(k, _)| *k == key) {
                Some((_, values)) => values.push(value.into()),
                None => terms.push((key, vec![value.into()])),
            }
        }
        for (key, _) in &terms {
            segments(key).try_for_each(validate_property)?;
        }
        Ok(Self { terms })
    }

    /// Parses CLI arguments (`key=value`, URL-encoded like `nextflow lineage find`).
    ///
    /// # Errors
    /// [`LineageError::InvalidQuery`] with the CLI's message (without its
    /// `Error searching for <arg>.` prefix and `-- Did you mean` hint) for an argument without
    /// `=`, a malformed `%` escape or an unknown property.
    pub fn parse_args<S: AsRef<str>>(
        args: impl IntoIterator<Item = S>,
    ) -> Result<Self, LineageError> {
        let mut pairs = Vec::new();
        for arg in args {
            let arg = arg.as_ref();
            let Some((key, value)) = arg.split_once('=') else {
                return Err(invalid(format!(
                    "Parameter {arg} doesn't contain '=' separator"
                )));
            };
            pairs.push((url_decode(key)?, url_decode(value)?));
        }
        Self::new(pairs)
    }

    /// The `(key, values)` terms, keys in first-seen order.
    pub fn terms(&self) -> &[(String, Vec<String>)] {
        &self.terms
    }

    /// Whether `record` matches every term, with the CLI's semantics: key `type` is the record
    /// kind; dot paths flatten arrays; null, `false`, `0`, `""` and empty arrays/objects never
    /// match; an array matches when it contains every value; a scalar matches one value by its
    /// Java string form.
    pub fn matches(&self, record: &Record) -> bool {
        let spec = record.spec();
        self.terms.iter().all(|(key, expected)| {
            let path: Vec<&str> = segments(key).collect();
            let value = if key == "type" {
                Value::String(record.kind().to_owned())
            } else {
                navigate(&spec, &path)
            };
            let int = matches!(path.last(), Some(&("size" | "maxIterations")));
            match &value {
                v if !truthy(v) => false,
                Value::Array(items) => {
                    let strings: Vec<Option<String>> =
                        items.iter().map(|v| java_string(v, int)).collect();
                    expected.iter().all(|e| strings.contains(&Some(e.clone())))
                }
                v => expected.len() == 1 && java_string(v, int).as_ref() == expected.first(),
            }
        })
    }
}

/// The non-empty dot segments of a key (Groovy `tokenize('.')`).
fn segments(key: &str) -> impl Iterator<Item = &str> {
    key.split('.').filter(|s| !s.is_empty())
}

fn invalid(message: String) -> LineageError {
    LineageError::InvalidQuery { message }
}

/// Fails unless `segment` is a lineage model property.
pub(crate) fn validate_property(segment: &str) -> Result<(), LineageError> {
    if PROPERTIES.split_whitespace().any(|p| p == segment) {
        Ok(())
    } else {
        Err(invalid(format!(
            "Property '{segment}' doesn't exist in the lineage model."
        )))
    }
}

/// Follows `path` through objects; through an array it collects each item's truthy result
/// (one result is unwrapped). Missing → `null`.
pub(crate) fn navigate(value: &Value, path: &[&str]) -> Value {
    path.iter()
        .fold(value.clone(), |current, key| sub_path(&current, key))
}

fn sub_path(current: &Value, key: &str) -> Value {
    match current {
        Value::Object(map) => map.get(key).cloned().unwrap_or(Value::Null),
        Value::Array(items) => {
            let mut found: Vec<Value> = items
                .iter()
                .map(|item| sub_path(item, key))
                .filter(truthy)
                .collect();
            match found.len() {
                0 => Value::Null,
                1 => found.pop().unwrap_or_default(),
                _ => Value::Array(found),
            }
        }
        _ => Value::Null,
    }
}

/// Groovy truth.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// The Java `toString` of a JSON value as Nextflow decodes it: integers for `int` fields,
/// `Double` elsewhere. Objects and nested arrays never equal a query value.
fn java_string(value: &Value, int: bool) -> Option<String> {
    match value {
        Value::Null => Some("null".to_owned()),
        Value::Bool(b) => Some(b.to_string()),
        Value::String(s) => Some(s.clone()),
        Value::Number(n) if int && (n.is_i64() || n.is_u64()) => Some(n.to_string()),
        Value::Number(n) => n.as_f64().map(java_double),
        // ponytail: Groovy would print `[a, b]` / `[k:v]`; nobody queries for that.
        Value::Array(_) | Value::Object(_) => None,
    }
}

/// Java `Double.toString`.
fn java_double(x: f64) -> String {
    let abs = x.abs();
    if abs == 0.0 || (1e-3..1e7).contains(&abs) {
        let s = format!("{x}");
        if s.contains('.') { s } else { s + ".0" }
    } else {
        let s = format!("{x:E}");
        match s.split_once('E') {
            Some((mantissa, exp)) if !mantissa.contains('.') => format!("{mantissa}.0E{exp}"),
            _ => s,
        }
    }
}

/// Java `URLDecoder.decode(s, UTF-8)`: `+` is a space, `%XX` a byte; invalid UTF-8 becomes U+FFFD.
// ponytail: Java's `Integer.parseInt` also accepts a sign (`%+1`, `%-0`); we reject it.
fn url_decode(s: &str) -> Result<String, LineageError> {
    let chars: Vec<char> = s.chars().collect();
    let mut bytes = Vec::with_capacity(s.len());
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        match c {
            '+' => bytes.push(b' '),
            '%' => {
                let (Some(&hi), Some(&lo)) = (chars.get(i + 1), chars.get(i + 2)) else {
                    return Err(invalid(
                        "URLDecoder: Incomplete trailing escape (%) pattern".to_owned(),
                    ));
                };
                let (Some(h), Some(l)) = (hi.to_digit(16), lo.to_digit(16)) else {
                    let index = usize::from(hi.is_ascii_hexdigit());
                    return Err(invalid(format!(
                        "URLDecoder: Illegal hex characters in escape (%) pattern - Error at index {index} in: \"{hi}{lo}\""
                    )));
                };
                bytes.extend(u8::try_from(h * 16 + l));
                i += 2;
            }
            c => bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
        }
        i += 1;
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    /// `(args, LIDs the CLI printed)`.
    type Case = (Vec<String>, Vec<String>);

    /// The records and cases of `tests/data/find_cli.json` (recorded from the Nextflow 26.10.7
    /// CLI, host paths scrubbed).
    fn cli_table() -> (Vec<(String, Record)>, Vec<Case>) {
        let data: Value =
            serde_json::from_str(include_str!("../tests/data/find_cli.json")).unwrap();
        let records = data["records"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(lid, raw)| {
                let bytes = serde_json::to_vec(raw).unwrap();
                (lid.clone(), Record::parse(&bytes, lid).unwrap())
            })
            .collect();
        let cases = data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| serde_json::from_value(json!([c["args"], c["lids"]])).unwrap())
            .collect();
        (records, cases)
    }

    #[test]
    fn matches_cli_semantics_table() {
        let (records, cases) = cli_table();
        assert!(records.len() >= 28 && cases.len() >= 37);
        let mut failures = Vec::new();
        for (args, expected) in cases {
            let query = FindQuery::parse_args(&args).unwrap();
            let found: Vec<String> = records
                .iter()
                .filter(|(_, r)| query.matches(r))
                .map(|(lid, _)| lid.clone())
                .collect();
            if found != expected {
                failures.push(format!("{args:?}: got {found:?}, CLI {expected:?}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn parse_args_errors_match_cli_messages() {
        // CLI text after `Error searching for <args[0]>. `, without the ` -- Did you mean` hint.
        let cases = [
            (vec!["type"], "Parameter type doesn't contain '=' separator"),
            (
                vec!["foo=bar"],
                "Property 'foo' doesn't exist in the lineage model.",
            ),
            (
                vec!["kind=TaskRun"],
                "Property 'kind' doesn't exist in the lineage model.",
            ),
            (
                vec!["config.runName=x"],
                "Property 'runName' doesn't exist in the lineage model.",
            ),
            (
                vec!["name=x", "metaClass=x"],
                "Property 'metaClass' doesn't exist in the lineage model.",
            ),
            (
                vec!["futureField=x"],
                "Property 'futureField' doesn't exist in the lineage model.",
            ),
            (
                vec!["name=%zz"],
                "URLDecoder: Illegal hex characters in escape (%) pattern - Error at index 0 in: \"zz\"",
            ),
            (
                vec!["name=%a"],
                "URLDecoder: Incomplete trailing escape (%) pattern",
            ),
            // Separator errors come before property errors, whatever the argument order.
            (
                vec!["foo=bar", "noeq"],
                "Parameter noeq doesn't contain '=' separator",
            ),
        ];
        for (args, message) in cases {
            match FindQuery::parse_args(&args) {
                Err(LineageError::InvalidQuery { message: m }) => {
                    assert_eq!(m, message, "{args:?}")
                }
                other => panic!("{args:?}: {other:?}"),
            }
        }
    }

    #[test]
    fn parse_args_decodes_aliases_and_accumulates() {
        let q = FindQuery::parse_args(["label=a+b", "labels=%C3%A9", "name=x=y", "workflowRun=%2"]);
        assert!(q.is_err());
        let q = FindQuery::parse_args(["label=a+b", "labels=%C3%A9", "name=x=y", "params..name=p"])
            .unwrap();
        let terms: Vec<(String, Vec<String>)> = vec![
            ("labels".into(), vec!["a b".into(), "é".into()]),
            ("name".into(), vec!["x=y".into()]),
            ("params..name".into(), vec!["p".into()]),
        ];
        assert_eq!(q.terms(), terms.as_slice());
        // `new` aliases and validates but does not decode.
        let q = FindQuery::new([("label", "a+b")]).unwrap();
        assert_eq!(
            q.terms(),
            [("labels".to_owned(), vec!["a+b".to_owned()])].as_slice()
        );
        assert!(FindQuery::new([("kind", "x")]).is_err());
        // An empty query matches every record, like the CLI without arguments.
        let any = Record::parse(br#"{"version":"lineage/v1beta1","kind":"Other"}"#, "x").unwrap();
        assert!(FindQuery::parse_args::<&str>([]).unwrap().matches(&any));
        assert!(FindQuery::parse_args(["type=Other"]).unwrap().matches(&any));
    }

    #[test]
    fn java_double_strings() {
        for (x, s) in [
            (12.0, "12.0"),
            (1.5, "1.5"),
            (0.001, "0.001"),
            (0.0001, "1.0E-4"),
            (12345678.0, "1.2345678E7"),
            (1e7, "1.0E7"),
            (-2.5e-7, "-2.5E-7"),
            (9999999.0, "9999999.0"),
        ] {
            assert_eq!(java_double(x), s);
        }
        let v = json!({"a": [{"b": 0}, {"b": [1, 2]}, {"b": "x"}, {"c": 1}], "n": {"size": 9}});
        assert_eq!(navigate(&v, &["a", "b"]), json!([[1, 2], "x"]));
        assert_eq!(navigate(&v, &["a", "c"]), json!(1));
        assert_eq!(navigate(&v, &["n", "missing", "x"]), Value::Null);
    }
}
