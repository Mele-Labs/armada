//! A project's glossary over the wire. `docs/concepts/feature-catalog.md`, *Glossary*.
//!
//! **Written by the Job that introduced a word, read by project.** A term carries the words that mean the same and
//! should not be used; a Job that uses one in a commit is flagged on its feature.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// A term a Job writes with `write_feature_sections`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryTermWrite {
    pub term: String,
    /// What it means, in a sentence.
    pub meaning: String,
    /// Words that mean the same and should not be used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not: Vec<String>,
}

/// One term of a project's glossary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryEntry {
    pub term: String,
    pub meaning: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not: Vec<String>,
    /// The Job that wrote it, or `seed` where Fleet's seed did.
    pub job: String,
    pub at: Instant,
}

/// `get_glossary`: a project's terms, in alphabetical order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Glossary {
    pub project: String,
    pub terms: Vec<GlossaryEntry>,
}
