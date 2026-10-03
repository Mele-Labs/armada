//! What a person decides about a Job at its approval gate, beyond the record's
//! own columns: how it lands, and which issue its words came from. Spike 022,
//! slice 4.
//!
//! **Each is a setting kept beside the Job, not a field of it**, the shape
//! `TierModels` took in slice 3: set after creation, read where it is used, and
//! absent on every Job approved before 23.8 — which is that Job doing what
//! every Job did then.

use alloc::string::String;
use alloc::vec::Vec;

use crate::envelope::Timestamp;
use crate::job::fields::{AcceptanceCriterion, Branch, Facts};
use crate::job::ids::Title;
use crate::job::status::JobStatus;
use crate::job::step::StepSeed;
use crate::job::workflow::FrozenWorkflow;

/// Whether the pull request is offered for review or parked as a draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PrMode {
    #[default]
    Ready,
    Draft,
}

impl PrMode {
    pub const ALL: &'static [PrMode] = &[PrMode::Ready, PrMode::Draft];

    pub fn as_wire(&self) -> &'static str {
        match self {
            PrMode::Ready => "ready",
            PrMode::Draft => "draft",
        }
    }

    pub fn from_wire(value: &str) -> Option<PrMode> {
        PrMode::ALL
            .iter()
            .copied()
            .find(|mode| mode.as_wire() == value)
    }
}

/// What has to happen before a Job counts as finished. **Only the two Fleet
/// honours**: `pr_merged` and `pr_opened` are refused at the approval (spike
/// 022, `landing.ts`'s `COMPLETE_WHEN_SERVED`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompleteWhen {
    /// Its last step is done, as every Job before slice 6 was.
    #[default]
    Delivered,
    /// Every Job it holds has had its pull request merged. Slice 6.
    AllMembersLanded,
}

impl CompleteWhen {
    pub const ALL: &'static [CompleteWhen] =
        &[CompleteWhen::Delivered, CompleteWhen::AllMembersLanded];

    pub fn as_wire(&self) -> &'static str {
        match self {
            CompleteWhen::Delivered => "delivered",
            CompleteWhen::AllMembersLanded => "all_members_landed",
        }
    }

    pub fn from_wire(value: &str) -> Option<CompleteWhen> {
        CompleteWhen::ALL
            .iter()
            .copied()
            .find(|when| when.as_wire() == value)
    }
}

/// How one Job's work reaches the repository, as approved.
///
/// **`None` on either branch is the Manifest's base**, which is not a branch
/// name by another spelling: a Manifest naming none lets the adapter infer
/// one, and a value copied in here would freeze a guess.
///
/// **Only what Fleet honours is here.** One branch and one pull request per
/// Job is the only unit slice 4 runs, so it is not a field: the approval
/// refuses any other (spike 022, `landing.ts`), and a field holding the one
/// legal value would be a setting nothing reads.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Landing {
    /// The branch the pull request is opened against.
    pub target: Option<Branch>,
    /// The branch the worktree is cut from. **Not `target`** where a person
    /// starts from an unmerged branch or lands in a long-lived one.
    pub from_ref: Option<Branch>,
    pub pr_mode: PrMode,
    pub complete_when: CompleteWhen,
}

impl Landing {
    /// Every Job before slice 4: cut from the base, landing in it, ready.
    pub fn as_ever() -> Landing {
        Landing::default()
    }
}

/// The issue a Job's request linked, and what Fleet has noticed about it since.
///
/// **One per Job at most**: `adapters::IssueLookup` resolves the one link
/// shape it knows, once, when the request arrives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueSource {
    /// How a person reads it: `armada#1162`.
    pub reference: String,
    /// The forge's own address for it, which is the only thing Bridge may open.
    pub url: String,
    /// When Fleet read it. What an edit is compared against.
    pub read_at: Timestamp,
    /// When the issue was last edited, where that is after `read_at`. **`None`
    /// is the ordinary case**: the issue still says what was read.
    pub moved_at: Option<Timestamp>,
}

impl IssueSource {
    /// Read just now, and not moved.
    pub fn read(reference: String, url: String, read_at: Timestamp) -> IssueSource {
        IssueSource {
            reference,
            url,
            read_at,
            moved_at: None,
        }
    }

    /// The same issue, edited at `edited_at` — **or `None` where that says
    /// nothing new**: no later than the read, or no later than the edit
    /// already noticed. A timestamp that will not parse says nothing either.
    pub fn edited(&self, edited_at: &Timestamp) -> Option<IssueSource> {
        let edited = edited_at.epoch_millis()?;
        let known = self.moved_at.as_ref().unwrap_or(&self.read_at);
        if edited <= known.epoch_millis()? {
            return None;
        }
        Some(IssueSource {
            moved_at: Some(edited_at.clone()),
            ..self.clone()
        })
    }
}

/// What a person's edit at the approval gate replaces on the record.
///
/// **Whole, not a diff.** `fleet::approving` reads the body against the Job
/// and hands this every value, changed or not, so the record is rewritten in
/// one move and cannot be left half on a new workflow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalEdit {
    pub title: Title,
    pub facts: Facts,
    /// The workflow with each step's gate as the person set it, and its steps.
    pub workflow: FrozenWorkflow,
    pub steps: Vec<StepSeed>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
}

/// Why an edit to a proposal was refused: the Job is not at its approval gate,
/// so what it is held to has already frozen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAtApproval {
    pub status: JobStatus,
}

impl core::fmt::Display for NotAtApproval {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            out,
            "the job is {}, not awaiting_approval, so what it runs has frozen",
            self.status.as_wire()
        )
    }
}

impl core::error::Error for NotAtApproval {}

/// A branch name read off a person, or `None` where it is blank.
pub fn branch_named(name: Option<&str>) -> Option<Branch> {
    name.and_then(|name| Branch::new(name.trim()).ok())
}

/// Every id in `held` and `sent`, read as `c<n>`, and the next one after the
/// largest. **Never an id already used**, even one a person removed: nothing
/// cites it yet, and a reader of the log should not meet two meanings.
pub fn next_criterion_number<'a>(ids: impl IntoIterator<Item = &'a str>) -> u32 {
    ids.into_iter()
        .filter_map(|id| id.strip_prefix('c')?.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

/// `c<n>`, the shape `fleet::drafting` mints at creation.
pub fn criterion_numbered(n: u32) -> String {
    alloc::format!("c{n}")
}
