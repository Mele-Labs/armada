//! What names an issue a Session works on: the forge's number, or a tracker's key.
//! `docs/concepts/session/what-is-kept.md`.
//!
//! **One rule, here, because both sides read it**: `record_plan` and `claim_issue` refuse what it
//! refuses, and Fleet holds an `issue` row only for what it reads.

use std::fmt;

use serde::{Deserialize, Deserializer};

/// An issue as the ledger holds it: digits for the forge's number, with no `#`, or a key such as
/// `ABC-12`. Nothing else is one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IssueReference(String);

impl IssueReference {
    /// `7`, `#7` or `ABC-12`, trimmed. `None` for anything else, a lowercase key included.
    pub fn read(written: &str) -> Option<IssueReference> {
        let written = written.trim();
        let digits = written.strip_prefix('#').unwrap_or(written);
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            return digits.parse::<u64>().ok().map(IssueReference::number);
        }
        is_key(written).then(|| IssueReference(written.to_string()))
    }

    pub fn number(number: u64) -> IssueReference {
        IssueReference(number.to_string())
    }

    /// As the ledger's `target` and a tracker's `read_issue` take it.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_number(&self) -> bool {
        self.0.bytes().all(|b| b.is_ascii_digit())
    }
}

/// As a person writes it: `#7`, `ABC-12`.
impl fmt::Display for IssueReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.is_number() {
            true => write!(f, "#{}", self.0),
            false => f.write_str(&self.0),
        }
    }
}

/// `[A-Z][A-Z0-9_]*-[0-9]+`.
fn is_key(text: &str) -> bool {
    let Some((project, number)) = text.split_once('-') else {
        return false;
    };
    project.starts_with(|c: char| c.is_ascii_uppercase())
        && project.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
}

/// An `issue` field as written, where a caller from before keys sends a JSON number.
pub(crate) fn number_or_text<'de, D: Deserializer<'de>>(from: D) -> Result<Option<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Number(u64),
        Text(String),
    }
    Ok(Option::<Written>::deserialize(from)?.map(|written| match written {
        Written::Number(number) => number.to_string(),
        Written::Text(text) => text,
    }))
}
