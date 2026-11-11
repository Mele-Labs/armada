//! The values a person sets for how work behaves, and the layer each one came from. **Only what a
//! layer sets is held**, so a key nobody spoke of falls through to the next layer.
//!
//! A [`Values`] is what a Project's file, Kit's `values.yml`, a Machine's settings or a repository's
//! Manifest say. Laying them over each other, the nearest winning, is `config::values`'s.

use alloc::collections::BTreeMap;
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
    /// The tracker a Project names for its issues and for the Jobs in it. **Only a Project's layer sets one**:
    /// Kit has none, and a repository's is its Manifest's own block, which is not held here.
    pub tracker: Option<TrackerNamed>,
    /// How the Operator reviews work before it reaches the person, a kind at a time. **A layer that sets it sets
    /// all three**: a kind it does not name is [`ReviewMode::Off`], and no layer setting it is the same as Off.
    pub reviews: Option<Reviews>,
}

/// What the Operator does with a Job's plan, pull request or walk before the person sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReviewMode {
    /// The item reaches the person as it is.
    #[default]
    Off,
    /// One reading call reviews it, and the item reaches the person with the findings.
    Review,
    /// As `Review`, and where the review finds nothing the Operator approves it itself.
    ReviewAndApprove,
}

impl ReviewMode {
    pub const ALL: &'static [ReviewMode] = &[ReviewMode::Off, ReviewMode::Review, ReviewMode::ReviewAndApprove];

    pub fn as_wire(self) -> &'static str {
        match self {
            ReviewMode::Off => "off",
            ReviewMode::Review => "review",
            ReviewMode::ReviewAndApprove => "review_and_approve",
        }
    }

    pub fn from_wire(word: &str) -> Option<ReviewMode> {
        ReviewMode::ALL.iter().copied().find(|mode| mode.as_wire() == word)
    }
}

/// What the Operator does with a walk. **Never approves**: a walk is the person's to look at, so the type has
/// no word for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WalkReview {
    #[default]
    Off,
    Review,
}

impl WalkReview {
    pub const ALL: &'static [WalkReview] = &[WalkReview::Off, WalkReview::Review];

    pub fn as_wire(self) -> &'static str {
        match self {
            WalkReview::Off => "off",
            WalkReview::Review => "review",
        }
    }

    pub fn from_wire(word: &str) -> Option<WalkReview> {
        WalkReview::ALL.iter().copied().find(|mode| mode.as_wire() == word)
    }
}

/// How each kind of work is reviewed. Everything is Off until a layer says otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reviews {
    pub plans: ReviewMode,
    pub pull_requests: ReviewMode,
    pub walks: WalkReview,
}

/// A tracker as a layer wrote it: the provider where it named one, and every other key as written, for that
/// provider to read. **No provider is the forge's own.** The same shape as the Manifest's `tracker:` block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrackerNamed {
    pub provider: Option<String>,
    pub keys: BTreeMap<String, String>,
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
