//! The prompts Fleet sends an agent, each one editable. **This table never
//! spells a prompt**: what ships is Fleet's, handed in through
//! [`Supplied`](super::Supplied), so the wording has one home and the
//! contract tests that hold it to `docs/contracts/agent-prompt.md` still read
//! it there.
//!
//! **A placeholder is filled by Fleet at the moment the prompt is built**, and
//! an override that drops one is refused like any other bad value: a brief
//! missing `{root}` is a scout that does not know where it is.

use super::{Applies, Entry, Kind, Section, Shipped, Words};

pub const PROMPT_DRONE_BASELINE: Words = Words("prompts.droneBaseline");
pub const PROMPT_SCOUT_ASK: Words = Words("prompts.scoutAsk");
pub const PROMPT_SCOUT_READ_IN: Words = Words("prompts.scoutReadIn");
pub const PROMPT_SCOUT_RESCUE: Words = Words("prompts.scoutRescue");
pub const PROMPT_PROPOSER: Words = Words("prompts.proposer");
pub const PROMPT_RETRO_JOB: Words = Words("prompts.retroJob");
pub const PROMPT_RETRO_SESSION: Words = Words("prompts.retroSession");
pub const PROMPT_RETRO_WHERE_FIXES_LAND: Words = Words("prompts.retroWhereFixesLand");
pub const PROMPT_RETRO_HOW_TO_WRITE: Words = Words("prompts.retroHowToWrite");
pub const PROMPT_RETRO_REVIEW: Words = Words("prompts.retroReview");
pub const PROMPT_RETRO_ASK: Words = Words("prompts.retroAsk");
pub const PROMPT_CROSSING_REDIRECT: Words = Words("prompts.crossingRedirect");
pub const PROMPT_CROSSING_OVERTAKEN: Words = Words("prompts.crossingOvertaken");
pub const PROMPT_CROSSING_CONFLICTS: Words = Words("prompts.crossingConflicts");
pub const PROMPT_HELM: Words = Words("prompts.helm");
pub const PROMPT_HELM_READ_ONLY: Words = Words("prompts.helmReadOnly");
pub const PROMPT_TRIGGER_REPAIR: Words = Words("prompts.triggerRepair");

/// Every prompt handle in the table, so a caller supplying or reading them
/// all cannot miss one.
pub fn prompt_keys() -> impl Iterator<Item = Words> {
    super::entries()
        .filter(|entry| matches!(entry.kind, Kind::Prompt(_)))
        .map(|entry| Words(entry.key))
}

/// The sub-heading a prompt is drawn under, inside the Prompts group. **In
/// the order a Drone meets them**: its brief, what it is told at a step
/// boundary and about other Jobs, the step's own terms, the turns injected
/// while it works, then the Judge that reads its work, and last the agents that
/// are not Drones. [`super::entries`] lays prompts out in this order whichever
/// table holds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptSection {
    DroneBrief,
    Crossing,
    AnotherJobsFix,
    OtherJobs,
    StepTerms,
    Review,
    InjectedTurns,
    PlanChanges,
    WorktreeRepair,
    SideRuns,
    Triggers,
    Judge,
    JudgeGaming,
    JudgeSecondReading,
    JudgeConvergence,
    JudgeWidening,
    Scouts,
    Proposer,
    Helm,
    Retros,
}

impl PromptSection {
    /// Every section, in the order Bridge draws them.
    pub const ORDER: [PromptSection; 20] = [
        PromptSection::DroneBrief,
        PromptSection::Crossing,
        PromptSection::AnotherJobsFix,
        PromptSection::OtherJobs,
        PromptSection::StepTerms,
        PromptSection::Review,
        PromptSection::InjectedTurns,
        PromptSection::PlanChanges,
        PromptSection::WorktreeRepair,
        PromptSection::SideRuns,
        PromptSection::Triggers,
        PromptSection::Judge,
        PromptSection::JudgeGaming,
        PromptSection::JudgeSecondReading,
        PromptSection::JudgeConvergence,
        PromptSection::JudgeWidening,
        PromptSection::Scouts,
        PromptSection::Proposer,
        PromptSection::Helm,
        PromptSection::Retros,
    ];

    /// Where the section stands in [`PromptSection::ORDER`]. Exhaustive, so a
    /// new section is placed before it compiles.
    pub const fn position(self) -> usize {
        match self {
            PromptSection::DroneBrief => 0,
            PromptSection::Crossing => 1,
            PromptSection::AnotherJobsFix => 2,
            PromptSection::OtherJobs => 3,
            PromptSection::StepTerms => 4,
            PromptSection::Review => 5,
            PromptSection::InjectedTurns => 6,
            PromptSection::PlanChanges => 7,
            PromptSection::WorktreeRepair => 8,
            PromptSection::SideRuns => 9,
            PromptSection::Triggers => 10,
            PromptSection::Judge => 11,
            PromptSection::JudgeGaming => 12,
            PromptSection::JudgeSecondReading => 13,
            PromptSection::JudgeConvergence => 14,
            PromptSection::JudgeWidening => 15,
            PromptSection::Scouts => 16,
            PromptSection::Proposer => 17,
            PromptSection::Helm => 18,
            PromptSection::Retros => 19,
        }
    }

    /// The sub-heading Bridge draws.
    pub fn title(self) -> &'static str {
        match self {
            PromptSection::DroneBrief => "Drone brief",
            PromptSection::Crossing => "Crossing a step",
            PromptSection::AnotherJobsFix => "Another Job's fix",
            PromptSection::OtherJobs => "Other Jobs",
            PromptSection::StepTerms => "Step terms",
            PromptSection::Review => "Review",
            PromptSection::InjectedTurns => "Injected turns",
            PromptSection::PlanChanges => "Plan changes",
            PromptSection::WorktreeRepair => "Worktree repair",
            PromptSection::SideRuns => "Side runs",
            PromptSection::Triggers => "Triggers",
            PromptSection::Judge => "Judge",
            PromptSection::JudgeGaming => "Judge: gaming",
            PromptSection::JudgeSecondReading => "Judge: second reading",
            PromptSection::JudgeConvergence => "Judge: convergence",
            PromptSection::JudgeWidening => "Judge: widening",
            PromptSection::Scouts => "Scouts",
            PromptSection::Proposer => "Proposer",
            PromptSection::Helm => "Helm",
            PromptSection::Retros => "Retros",
        }
    }
}

/// One prompt: what it is, and the placeholders an override must keep.
pub(super) const fn prompt(
    section: PromptSection,
    key: Words,
    title: &'static str,
    description: &'static str,
    keeps: &'static [&'static str],
) -> Entry {
    Entry {
        key: key.0,
        section: Section::Prompt(section),
        title,
        description,
        kind: Kind::Prompt(keeps),
        shipped: Shipped::Supplied,
        applies: Applies::Live,
        row: None,
        env: None,
    }
}

pub(super) const PROMPTS: &[Entry] = &[
    prompt(
        PromptSection::DroneBrief,
        PROMPT_DRONE_BASELINE,
        "Drone baseline",
        "The first block of every Drone's brief: how it works and how it reports. The Job, the step and the checks follow it, written by Fleet.",
        &[],
    ),
    prompt(
        PromptSection::Scouts,
        PROMPT_SCOUT_ASK,
        "A question about the code",
        "What a scout is told before a question about a repository. {root} is the checkout it reads. The question follows it.",
        &["root"],
    ),
    prompt(
        PromptSection::Scouts,
        PROMPT_SCOUT_READ_IN,
        "Reading a link in",
        "What a scout is told before a pasted link's text. {source} says what was fetched and {root} is the checkout. The fetched text follows it.",
        &["source", "root"],
    ),
    prompt(
        PromptSection::Scouts,
        PROMPT_SCOUT_RESCUE,
        "Reading stranded work",
        "What a scout is told before reading work an agent left in a worktree. {root}, {branch}, {commit} and {base} say where the work is. What git read follows it.",
        &["root", "branch", "commit", "base"],
    ),
    prompt(
        PromptSection::Proposer,
        PROMPT_PROPOSER,
        "Job proposer",
        "What the proposer is asked when a request becomes Jobs. {request} is the request, {workflows} the workflows held and {models} the models this machine runs. Its answer is read by field name, so keep the field names.",
        &["request", "workflows", "models"],
    ),
    prompt(
        PromptSection::Retros,
        PROMPT_RETRO_JOB,
        "The retro of a Job",
        "What the retro of a finished Job is asked. {record} is the Job's record, and {where_fixes_land} and {how_to_write} are the two prompts below.",
        &["record", "where_fixes_land", "how_to_write"],
    ),
    prompt(
        PromptSection::Retros,
        PROMPT_RETRO_SESSION,
        "The retro of a session",
        "What the retro of a session is asked. {name} is the session's name, {record} its record, and {where_fixes_land} and {how_to_write} are the two prompts below.",
        &["name", "record", "where_fixes_land", "how_to_write"],
    ),
    prompt(
        PromptSection::Retros,
        PROMPT_RETRO_WHERE_FIXES_LAND,
        "Where a fix lands",
        "How both retros are told to say where each fix lands. Fleet reads the three places back by name.",
        &[],
    ),
    prompt(
        PromptSection::Retros,
        PROMPT_RETRO_HOW_TO_WRITE,
        "How to write an item",
        "How both retros are told to write an item's title, what and fix.",
        &[],
    ),
    prompt(
        PromptSection::Retros,
        PROMPT_RETRO_REVIEW,
        "The review of the open items",
        "What the model is asked when the open retro items are reviewed together. {items} is the list of open items. Its answer is read by field name, so keep the field names.",
        &["items"],
    ),
    prompt(
        PromptSection::Retros,
        PROMPT_RETRO_ASK,
        "A question about an item",
        "What the model is asked when a person asks about one retro item. {item} is the item, {rows} the record rows it cites, {history} the earlier turns and {question} the question.",
        &["item", "rows", "history", "question"],
    ),
    prompt(
        PromptSection::Crossing,
        PROMPT_CROSSING_REDIRECT,
        "A person's note",
        "The block a new Drone reads when a person left a note before it started. {note} is the note, word for word.",
        &["note"],
    ),
    prompt(
        PromptSection::Crossing,
        PROMPT_CROSSING_OVERTAKEN,
        "A sibling landed",
        "The block a Drone reads when a Job from the same request landed while this one was between steps. {title} is that Job's title and {claimed} what its evidence said.",
        &["title", "claimed"],
    ),
    prompt(
        PromptSection::Crossing,
        PROMPT_CROSSING_CONFLICTS,
        "Sent back for conflicts",
        "The block a Drone reads when Fleet sent the work back because the base moved and the branch conflicts with it.",
        &[],
    ),
    prompt(
        PromptSection::Helm,
        PROMPT_HELM,
        "Helm",
        "What a Helm session is told when it starts, where Helm can act. {manifest} and {folder} name the repository, {reserved} and {unasked} the calls Fleet keeps or allows unasked, and {workflow_fields} and {workflow_sample} what a workflow is written in.",
        &["manifest", "folder", "reserved", "unasked", "workflow_fields", "workflow_sample"],
    ),
    prompt(
        PromptSection::Helm,
        PROMPT_HELM_READ_ONLY,
        "Helm, read only",
        "What a Helm session is told when it starts, where Helm only reads. {manifest} and {folder} name the repository, and {workflow_fields} and {workflow_sample} what a workflow is written in.",
        &["manifest", "folder", "workflow_fields", "workflow_sample"],
    ),
    prompt(
        PromptSection::Triggers,
        PROMPT_TRIGGER_REPAIR,
        "Repairing a Trigger",
        "What a Drone is told under the heading naming a failed Trigger. {command} is what it runs, {exited} says how it ended and {printed} is what it printed. {trigger}, its name, may be used too.",
        &["command", "exited", "printed"],
    ),
];
