//! The librarian over the wire: the act that runs it on a press, and where it stands on the features listing.
//! `docs/concepts/feature-catalog.md`, *Librarian*.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// One part of the librarian's upkeep. **A closed set**, named in `features.librarianParts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibrarianPart {
    FloorPlans,
    /// The docs and tickets Jobs read.
    Links,
    Decisions,
    Health,
    Drift,
    /// Retire, Planned and dead links.
    Retire,
    Glossary,
    SinceYouLooked,
    Gists,
    /// One picture a feature, of the screen it is tied to, on the app running now.
    Pictures,
}

impl LibrarianPart {
    pub const ALL: [LibrarianPart; 10] = [
        LibrarianPart::FloorPlans,
        LibrarianPart::Links,
        LibrarianPart::Decisions,
        LibrarianPart::Health,
        LibrarianPart::Drift,
        LibrarianPart::Retire,
        LibrarianPart::Glossary,
        LibrarianPart::SinceYouLooked,
        LibrarianPart::Gists,
        LibrarianPart::Pictures,
    ];

    /// The name the setting and the run record spell it by.
    pub const fn name(self) -> &'static str {
        match self {
            LibrarianPart::FloorPlans => "floor_plans",
            LibrarianPart::Links => "links",
            LibrarianPart::Decisions => "decisions",
            LibrarianPart::Health => "health",
            LibrarianPart::Drift => "drift",
            LibrarianPart::Retire => "retire",
            LibrarianPart::Glossary => "glossary",
            LibrarianPart::SinceYouLooked => "since_you_looked",
            LibrarianPart::Gists => "gists",
            LibrarianPart::Pictures => "pictures",
        }
    }

    pub fn from_name(name: &str) -> Option<LibrarianPart> {
        Self::ALL.into_iter().find(|one| one.name() == name)
    }

    /// Whether the part writes with a model, so a night's share of calls bounds it.
    pub const fn spends_model_share(self) -> bool {
        matches!(self, LibrarianPart::SinceYouLooked | LibrarianPart::Gists | LibrarianPart::Drift)
    }
}

/// `run_librarian`: the librarian runs the enabled parts for one project, now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunLibrarian {
    /// A project's id.
    pub project: String,
}

/// What `run_librarian` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianRan {
    pub project: String,
    pub ran_at: Instant,
    /// The parts that ran, in the order they ran.
    pub ran: Vec<LibrarianPart>,
    /// The newest merged pull request the run read through. Absent where the project holds none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_through: Option<u64>,
    /// The parts the run left out because the night's share of model calls was spent. Absent where none was.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_for_share: Vec<LibrarianPart>,
    /// Stop was pressed before the run ended, so the steps after the one running then were skipped. Absent where it was not.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stopped: bool,
}

/// One step a run takes, as the Catch up card lists it before a press. Only steps a run does: no step for what a page
/// reads live.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianStep {
    /// `read_merges`, a model part's name such as `since_you_looked`, or `read_through`. `librarian.step` names it.
    pub id: String,
    pub title: String,
    /// One line on what it will read or spend now.
    pub detail: String,
    /// Whether it spends a model call from the night's share. Free where not.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub model: bool,
}

/// Where one step of a run stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibrarianStepState {
    Running,
    Done,
    Skipped,
    Failed,
}

/// `librarian.step`: one step of a run started or ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianStepMoved {
    pub project: String,
    /// The run, by the instant it started, which `LibrarianRan.ran_at` repeats.
    pub run: Instant,
    /// The step's `id`.
    pub step: String,
    pub state: LibrarianStepState,
    /// One line of what it did, or why it was skipped or failed. Absent while it runs.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub said: String,
}

/// `stop_librarian`: the project's running run stops before its next step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopLibrarian {
    /// A project's id.
    pub project: String,
}

/// What `stop_librarian` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianStopping {
    pub project: String,
    /// A run was going and will stop before its next step. False where none was.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stopping: bool,
}

/// Where a project's librarian stands, on `list_features`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianState {
    /// When it last ran. Absent where it never has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<Instant>,
    /// The newest merged pull request its last run read through. Absent where it never ran or the project held none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_through: Option<u64>,
    /// The merged pull requests since that run, oldest first. Every merge Fleet kept for the project's features where it never ran.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merged_since: Vec<u64>,
    /// The steps a run would take now, in order, each free or model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<LibrarianStep>,
    /// The run going now, so a Bridge that reads mid-run redraws its steps and Stop. Absent where none is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<LibrarianRunning>,
}

/// A run going now, as `librarian.step` has told it so far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianRunning {
    /// The run, by the instant it started, as `librarian.step` names it.
    pub run: Instant,
    /// The steps that have started or ended so far, in order. A step not yet reached is not listed.
    pub steps: Vec<LibrarianStepNow>,
}

/// Where one step of a run going now stands: the latest `librarian.step` told of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianStepNow {
    pub step: String,
    pub state: LibrarianStepState,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub said: String,
}
