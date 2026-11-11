//! The values a person sets for how work behaves, and the layer each one came from. **Only what a
//! layer sets is held**, so a key nobody spoke of falls through to the next layer.
//!
//! A [`Values`] is what a Project's file, Kit's `values.yml`, a Machine's settings or a repository's
//! Manifest say. Laying them over each other, the nearest winning, is `config::values`'s.

use alloc::string::String;

use crate::policy::AutoMerge;
use crate::workflow::PrMode;

/// The values one layer sets. `None` is that layer saying nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Values {
    pub pr_mode: Option<PrMode>,
    pub auto_merge: Option<AutoMerge>,
    /// The model an agent run that reviews work runs on, as the agent spells it.
    pub review_model: Option<String>,
    pub look: Look,
}

/// How Bridge looks. Fleet keeps and serves the words, and Bridge decides what each names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Look {
    pub layout: Option<String>,
    pub theme: Option<String>,
    /// `compact`, `default` or `roomy`.
    pub density: Option<String>,
}

impl Values {
    /// Whether the layer sets nothing at all.
    pub fn is_empty(&self) -> bool {
        *self == Values::default()
    }
}

/// Where a value came from, from the furthest layer to the nearest. **The nearest that sets a key
/// wins**: a Job's own over its Project's, a Project's over its repository's, then Kit's, the
/// Machine's and last the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    Default,
    Machine,
    Kit,
    Repository,
    Project,
    Job,
}

impl Layer {
    pub const ALL: &'static [Layer] = &[Layer::Default, Layer::Machine, Layer::Kit, Layer::Repository, Layer::Project, Layer::Job];

    pub fn as_wire(self) -> &'static str {
        match self {
            Layer::Default => "default",
            Layer::Machine => "machine",
            Layer::Kit => "kit",
            Layer::Repository => "repository",
            Layer::Project => "project",
            Layer::Job => "job",
        }
    }

    pub fn from_wire(word: &str) -> Option<Layer> {
        Layer::ALL.iter().copied().find(|layer| layer.as_wire() == word)
    }
}
