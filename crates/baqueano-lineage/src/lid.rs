//! Lineage IDs (`lid://<hash>[/<path>][#<fragment>]`).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize, Serializer};

use crate::LineageError;

const SCHEME: &str = "lid://";
const MAX_LEN: usize = 4096;
const MAX_HASH: usize = 128;

/// A parsed lineage ID: `lid://<hash>[/<path>][#<fragment>]`.
///
/// Ordered, compared and hashed by its canonical string; (de)serialized as that string.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct Lid {
    // `raw` first: the derived ordering is the canonical string's ordering.
    raw: String,
    hash: String,
    path: Option<String>,
    fragment: Option<String>,
}

impl Lid {
    /// Parses and validates a lineage ID.
    ///
    /// # Errors
    /// [`LineageError::InvalidLid`] naming the input when it is not a valid `lid://` URI.
    pub fn parse(s: &str) -> Result<Self, LineageError> {
        let invalid = |reason: &str| LineageError::InvalidLid {
            input: s.to_owned(),
            reason: reason.to_owned(),
        };
        if s.len() > MAX_LEN {
            return Err(invalid("longer than 4096 bytes"));
        }
        let rest = s
            .strip_prefix(SCHEME)
            .ok_or_else(|| invalid("missing 'lid://' prefix"))?;
        let (rest, fragment) = match rest.split_once('#') {
            Some((rest, frag)) => (rest, Some(frag)),
            None => (rest, None),
        };
        let (hash, path) = match rest.split_once('/') {
            Some((hash, path)) => (hash, Some(path)),
            None => (rest, None),
        };
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if hash.is_empty() || hash.len() > MAX_HASH || !hash.chars().all(hex) {
            return Err(invalid("the hash must be 1-128 lowercase hex characters"));
        }
        let bad_seg = |seg: &str| {
            seg.is_empty()
                || seg == "."
                || seg == ".."
                || seg
                    .chars()
                    .any(|c| matches!(c, '\\' | '?') || c.is_control())
        };
        if path.is_some_and(|p| p.split('/').any(bad_seg)) {
            return Err(invalid("invalid path segment"));
        }
        let prop = |seg: &str| {
            !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        };
        if fragment.is_some_and(|f| !f.split('.').all(prop)) {
            return Err(invalid("fragment segments must match [A-Za-z0-9_]+"));
        }
        Ok(Self {
            raw: s.to_owned(),
            hash: hash.to_owned(),
            path: path.map(str::to_owned),
            fragment: fragment.map(str::to_owned),
        })
    }

    /// The record hash (first segment).
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The file path below the hash, e.g. `upper/upper.txt`.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// The fragment after `#`, e.g. `output` or `checksum.value`.
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// The store key of the record this LID names: `<hash>#output` for `lid://<hash>#output`,
    /// else `<hash>[/<path>]`.
    pub fn record_key(&self) -> String {
        match (&self.path, self.fragment.as_deref()) {
            (None, Some("output")) => format!("{}#output", self.hash),
            (None, _) => self.hash.clone(),
            (Some(path), _) => format!("{}/{path}", self.hash),
        }
    }
}

impl fmt::Display for Lid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl FromStr for Lid {
    type Err = LineageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for Lid {
    type Error = LineageError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl TryFrom<String> for Lid {
    type Error = LineageError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl Serialize for Lid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: &str = "6d6d78525bd5066888a8696fbe711344";

    #[test]
    fn parses_run_task_and_file_lids() {
        let run = Lid::parse(&format!("lid://{H}")).unwrap();
        assert_eq!((run.hash(), run.path(), run.fragment()), (H, None, None));
        let file: Lid = format!("lid://{H}/upper/upper.txt").parse().unwrap();
        assert_eq!(file.hash(), H);
        assert_eq!(file.path(), Some("upper/upper.txt"));
        assert_eq!(file.to_string(), format!("lid://{H}/upper/upper.txt"));
        assert_eq!(Lid::try_from("lid://ab").unwrap().to_string(), "lid://ab");
    }

    #[test]
    fn parses_output_fragment() {
        let lid = Lid::parse(&format!("lid://{H}#output")).unwrap();
        assert_eq!((lid.path(), lid.fragment()), (None, Some("output")));
        assert_eq!(lid.to_string(), format!("lid://{H}#output"));
        let nested = Lid::parse(&format!("lid://{H}/a.txt#checksum.value")).unwrap();
        assert_eq!(nested.path(), Some("a.txt"));
        assert_eq!(nested.fragment(), Some("checksum.value"));
    }

    #[test]
    fn rejects_invalid_lids() {
        let long = format!("lid://{H}/{}", "a".repeat(4100));
        for bad in [
            "foo",
            "lid://",
            "lid:///",
            "lid://ABC",
            "lid://h",
            "lid://ab/../x",
            "lid://ab/./x",
            "lid://ab/a//b",
            "lid://ab?x=1",
            "lid://ab/",
            "lid://ab/a\\b",
            "lid://ab/a?b",
            "lid://ab#",
            "lid://ab#a..b",
            "lid://ab#a-b",
            "lid://ab/a#b/c",
            "lid://ab/a\nb",
            "LID://ab",
            &long,
        ] {
            match Lid::parse(bad) {
                Err(LineageError::InvalidLid { input, .. }) => assert_eq!(input, bad),
                other => panic!("{bad:?} gave {other:?}"),
            }
        }
        assert!(Lid::parse(&format!("lid://{}", "a".repeat(129))).is_err());
    }

    #[test]
    fn record_key_maps_output_fragment() {
        let key = |s: &str| Lid::parse(s).unwrap().record_key();
        assert_eq!(key("lid://ab#output"), "ab#output");
        assert_eq!(key("lid://ab#params"), "ab");
        assert_eq!(key("lid://ab/p/q.txt#size"), "ab/p/q.txt");
        assert_eq!(key("lid://ab/p#output"), "ab/p");
    }

    #[test]
    fn serializes_as_string() {
        let lid = Lid::parse("lid://ab/x.txt").unwrap();
        let json = serde_json::to_string(&lid).unwrap();
        assert_eq!(json, r#""lid://ab/x.txt""#);
        assert_eq!(serde_json::from_str::<Lid>(&json).unwrap(), lid);
        assert!(serde_json::from_str::<Lid>(r#""nope""#).is_err());
        assert!(Lid::parse("lid://ab").unwrap() < Lid::parse("lid://ab#output").unwrap());
    }
}
