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
    /// Whether a Job that a Job or the Operator starts begins at once or waits on the person's start card.
    pub agent_starts: Option<AgentStarts>,
    /// How a Job in the Project lands its work. **Unset at every layer is the delivery question**, put to the person
    /// once and kept as the Project's value.
    pub delivery: Option<DeliverySetting>,
}

/// How a Job's work lands, as a Project or Kit sets it. `Ask` is the person saying the question is theirs to put each time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliverySetting {
    /// Open a pull request, watch ci and merge it.
    AutoLand,
    /// Open a pull request and leave the merge to the person.
    OpenPr,
    /// Keep the work on the branch; no pull request.
    KeepLocal,
    /// Put the delivery question to the person, and keep nothing from the answer.
    Ask,
}

impl DeliverySetting {
    pub const ALL: &'static [DeliverySetting] =
        &[DeliverySetting::AutoLand, DeliverySetting::OpenPr, DeliverySetting::KeepLocal, DeliverySetting::Ask];

    pub fn as_wire(self) -> &'static str {
        match self {
            DeliverySetting::AutoLand => "auto_land",
            DeliverySetting::OpenPr => "open_pr",
            DeliverySetting::KeepLocal => "keep_local",
            DeliverySetting::Ask => "ask",
        }
    }

    pub fn from_wire(word: &str) -> Option<DeliverySetting> {
        DeliverySetting::ALL.iter().copied().find(|setting| setting.as_wire() == word)
    }
}

/// What happens to a Job that an agent starts: a Job one starts, a lifecycle rule that starts one, or the Operator.
/// **A Job the person starts himself is never held by this.**
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentStarts {
    /// The Job begins running.
    #[default]
    AtOnce,
    /// The Job waits on the person's start card.
    AskMe,
}

impl AgentStarts {
    pub const ALL: &'static [AgentStarts] = &[AgentStarts::AtOnce, AgentStarts::AskMe];

    pub fn as_wire(self) -> &'static str {
        match self {
            AgentStarts::AtOnce => "at_once",
            AgentStarts::AskMe => "ask_me",
        }
    }

    pub fn from_wire(word: &str) -> Option<AgentStarts> {
        AgentStarts::ALL.iter().copied().find(|mode| mode.as_wire() == word)
    }
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
