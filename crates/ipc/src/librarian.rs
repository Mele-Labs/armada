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
}

impl LibrarianPart {
    pub const ALL: [LibrarianPart; 9] = [
        LibrarianPart::FloorPlans,
        LibrarianPart::Links,
        LibrarianPart::Decisions,
        LibrarianPart::Health,
        LibrarianPart::Drift,
        LibrarianPart::Retire,
        LibrarianPart::Glossary,
        LibrarianPart::SinceYouLooked,
        LibrarianPart::Gists,
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
}
