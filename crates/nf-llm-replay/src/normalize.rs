//! Request normalization: canonical JSON body and the SHA-256 cassette key.

use std::fmt::Write as _;

use regex::{NoExpand, Regex};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Top-level body fields that never take part in the key.
const DEFAULT_VOLATILE_FIELDS: [&str; 3] = ["user", "metadata", "prompt_cache_key"];
/// Patterns replaced in every string value (pi appends the task work dir to its prompt).
const DEFAULT_VOLATILE_PATTERNS: [&str; 1] = [r"(?s)<cwd>.*?</cwd>"];
/// Replacement for every volatile pattern match.
const VOLATILE: &str = "<volatile/>";

/// Turns an HTTP request into a canonical body and a stable cassette key.
#[derive(Debug)]
pub struct Normalizer {
    patterns: Vec<Regex>,
}

/// Why a request could not be normalized.
#[derive(Debug, thiserror::Error)]
pub enum NormalizeError {
    /// A non-empty body that is not JSON.
    #[error("request body is not valid JSON: {0}")]
    Body(serde_json::Error),
    /// A volatile pattern that is not a valid regular expression.
    #[error("invalid volatile pattern `{pattern}`: {source}")]
    Pattern {
        /// The pattern as given.
        pattern: String,
        /// Why it does not compile.
        source: regex::Error,
    },
}

/// A normalized request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Normalized {
    /// Lowercase hex SHA-256 of `METHOD + " " + PATH + "\n" + canonical`.
    pub key: String,
    /// Canonical JSON body; empty when the request body is empty.
    pub canonical: String,
}

impl Normalizer {
    /// A normalizer with the default volatile rules.
    pub fn new() -> Result<Self, NormalizeError> {
        let patterns = DEFAULT_VOLATILE_PATTERNS
            .iter()
            .map(|p| {
                Regex::new(p).map_err(|source| NormalizeError::Pattern {
                    pattern: (*p).to_owned(),
                    source,
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { patterns })
    }

    /// Normalizes one request. The query string of `path` is ignored; headers never matter.
    pub fn normalize(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<Normalized, NormalizeError> {
        let path = path.split_once('?').map_or(path, |(p, _)| p);
        let mut canonical = String::new();
        if !body.is_empty() {
            let mut value: Value = serde_json::from_slice(body).map_err(NormalizeError::Body)?;
            if let Value::Object(map) = &mut value {
                for field in DEFAULT_VOLATILE_FIELDS {
                    map.remove(field);
                }
            }
            self.scrub(&mut value);
            write_canonical(&value, &mut canonical);
        }
        let digest = Sha256::new()
            .chain_update(method)
            .chain_update(" ")
            .chain_update(path)
            .chain_update("\n")
            .chain_update(&canonical)
            .finalize();
        let key = digest.iter().fold(String::with_capacity(64), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        });
        Ok(Normalized { key, canonical })
    }

    /// Replaces volatile pattern matches in every string value (never in object keys).
    fn scrub(&self, value: &mut Value) {
        match value {
            Value::String(s) => {
                for re in &self.patterns {
                    if let std::borrow::Cow::Owned(new) = re.replace_all(s, NoExpand(VOLATILE)) {
                        *s = new;
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|v| self.scrub(v)),
            Value::Object(map) => map.values_mut().for_each(|v| self.scrub(v)),
            _ => {}
        }
    }
}

/// Compact JSON with object keys sorted by byte order at every depth, independent of any
/// `serde_json` map-ordering feature.
fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut members: Vec<_> = map.iter().collect();
            members.sort_unstable_by(|a, b| a.0.cmp(b.0));
            out.push('{');
            for (i, (k, v)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(k.clone()).to_string());
                out.push(':');
                write_canonical(v, out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(v, out);
            }
            out.push(']');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n() -> Normalizer {
        Normalizer::new().unwrap()
    }

    fn key(body: &str) -> String {
        n().normalize("POST", "/v1/chat/completions", body.as_bytes())
            .unwrap()
            .key
    }

    #[test]
    fn object_keys_are_sorted_recursively() {
        let out = n()
            .normalize(
                "POST",
                "/p",
                r#"{"b":1,"a":{"z":[{"y":1,"x":2}],"c":"\u00e9\"q"},"A":null}"#.as_bytes(),
            )
            .unwrap();
        assert_eq!(
            out.canonical,
            r#"{"A":null,"a":{"c":"é\"q","z":[{"x":2,"y":1}]},"b":1}"#
        );
    }

    #[test]
    fn whitespace_and_key_order_do_not_change_key() {
        let a = key(r#"{"model":"m","messages":[{"role":"user","content":"hi"}],"n":1.5}"#);
        let b = key(
            "{ \"n\" : 1.5,\n \"messages\" : [ { \"content\":\"hi\", \"role\":\"user\" } ],\t\"model\":\"m\" }",
        );
        assert_eq!(a, b);
        assert_ne!(
            a,
            key(r#"{"model":"m","messages":[{"role":"user","content":"ho"}],"n":1.5}"#)
        );
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
    }

    #[test]
    fn empty_body_is_keyed_as_empty_string() {
        let out = n().normalize("GET", "/v1/models", b"").unwrap();
        assert_eq!(out.canonical, "");
        // printf 'GET /v1/models\n' | sha256sum
        assert_eq!(
            out.key,
            "31859108c0d9d31c57940415975dd0e647887ce489ff7a1deb44eee760d3216f"
        );
        assert_ne!(
            out.key,
            n().normalize("GET", "/v1/models", b"{}").unwrap().key
        );
    }

    #[test]
    fn non_json_body_is_rejected() {
        let err = n().normalize("POST", "/p", b"not json").unwrap_err();
        assert!(matches!(err, NormalizeError::Body(_)));
        assert!(
            err.to_string()
                .starts_with("request body is not valid JSON")
        );
    }

    #[test]
    fn default_volatile_fields_do_not_change_key() {
        let base = key(r#"{"model":"m","input":"hi"}"#);
        assert_eq!(base, key(r#"{"model":"m","input":"hi","user":"u1"}"#));
        assert_eq!(
            base,
            key(r#"{"model":"m","input":"hi","metadata":{"user_id":"x"}}"#)
        );
        assert_eq!(
            base,
            key(r#"{"model":"m","input":"hi","prompt_cache_key":"0193-abc"}"#)
        );
        assert_eq!(
            key(r#"{"model":"m","input":"hi","prompt_cache_key":"a","user":"b"}"#),
            key(r#"{"model":"m","input":"hi","prompt_cache_key":"c","user":"d"}"#)
        );
        assert_ne!(base, key(r#"{"model":"m","input":"hi","store":true}"#));
    }

    #[test]
    fn nested_field_named_like_a_volatile_field_is_kept() {
        let tool = |t: &str| {
            format!(
                r#"{{"tools":[{{"function":{{"parameters":{{"properties":{{"user":{{"type":"{t}"}}}}}}}}}}]}}"#
            )
        };
        let out = n()
            .normalize("POST", "/p", tool("string").as_bytes())
            .unwrap();
        assert!(out.canonical.contains(r#""user":{"type":"string"}"#));
        assert_ne!(key(&tool("string")), key(&tool("integer")));
    }

    #[test]
    fn cwd_block_is_replaced_in_every_string_value() {
        let body = |dir: &str| {
            format!(
                r#"{{"input":[{{"role":"developer","content":"Be concise.\n\n<cwd>\n{dir}\n</cwd>"}}],"system":[{{"text":"a <cwd>{dir}</cwd> b <cwd>x</cwd>"}}],"<cwd>k</cwd>":1}}"#
            )
        };
        let a = n()
            .normalize("POST", "/p", body("/w/3a/35").as_bytes())
            .unwrap();
        let b = n()
            .normalize("POST", "/p", body("/w/93/7d").as_bytes())
            .unwrap();
        assert_eq!(a.key, b.key);
        assert_eq!(
            a.canonical,
            r#"{"<cwd>k</cwd>":1,"input":[{"content":"Be concise.\n\n<volatile/>","role":"developer"}],"system":[{"text":"a <volatile/> b <volatile/>"}]}"#
        );
    }

    #[test]
    fn unterminated_cwd_block_is_left_unchanged() {
        let out = n()
            .normalize("POST", "/p", br#"{"content":"see <cwd> without end"}"#)
            .unwrap();
        assert_eq!(out.canonical, r#"{"content":"see <cwd> without end"}"#);
    }

    #[test]
    fn query_string_is_not_part_of_key() {
        let body = br#"{"model":"claude"}"#;
        let plain = n().normalize("POST", "/v1/messages", body).unwrap();
        let beta = n()
            .normalize("POST", "/v1/messages?beta=true", body)
            .unwrap();
        let gemini = n()
            .normalize("POST", "/v1/messages?key=secret&x=1", body)
            .unwrap();
        assert_eq!(plain, beta);
        assert_eq!(plain, gemini);
    }

    #[test]
    fn method_and_path_are_part_of_key() {
        let body = br#"{"model":"m"}"#;
        let k = |m: &str, p: &str| n().normalize(m, p, body).unwrap().key;
        let base = k("POST", "/v1/chat/completions");
        assert_ne!(base, k("PUT", "/v1/chat/completions"));
        assert_ne!(base, k("POST", "/v1/responses"));
        // Path is used exactly as received: no percent-decoding.
        assert_ne!(k("POST", "/v1/a%20b"), k("POST", "/v1/a b"));
        // printf 'POST /v1/chat/completions\n{"model":"m"}' | sha256sum
        assert_eq!(
            base,
            "e4ce5776b0ae2cef282d8a29f297ce6725fc68b73dc8197ea3bf692581387e68"
        );
    }
}
