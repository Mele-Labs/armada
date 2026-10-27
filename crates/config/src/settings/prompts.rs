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

pub const PROMPT_SCOUT_ASK: Words = Words("prompts.scoutAsk");
pub const PROMPT_SCOUT_READ_IN: Words = Words("prompts.scoutReadIn");
pub const PROMPT_SCOUT_RESCUE: Words = Words("prompts.scoutRescue");
pub const PROMPT_RETRO_SESSION: Words = Words("prompts.retroSession");
pub const PROMPT_RETRO_WHERE_FIXES_LAND: Words = Words("prompts.retroWhereFixesLand");
pub const PROMPT_RETRO_HOW_TO_WRITE: Words = Words("prompts.retroHowToWrite");
pub const PROMPT_RETRO_REVIEW: Words = Words("prompts.retroReview");
pub const PROMPT_RETRO_ASK: Words = Words("prompts.retroAsk");
pub const PROMPT_LOGBOOK_DISTIL: Words = Words("prompts.logbookDistil");
pub const PROMPT_TEACHING_READ: Words = Words("prompts.teachingRead");
pub const PROMPT_HELM: Words = Words("prompts.helm");
pub const PROMPT_HELM_READ_ONLY: Words = Words("prompts.helmReadOnly");
pub const PROMPT_HELM_ROUTE: Words = Words("prompts.helmRoute");
pub const PROMPT_HELM_VOICE: Words = Words("prompts.helmVoice");
pub const PROMPT_AUTOPILOT_SENTENCE: Words = Words("prompts.autopilotSentence");
pub const PROMPT_AUTOPILOT_PICKER: Words = Words("prompts.autopilotPicker");
pub const PROMPT_TEST_MAP_READING: Words = Words("prompts.testMapReading");
pub const PROMPT_WORKFLOW_CHANGE: Words = Words("prompts.workflowChange");

/// Every prompt handle in the table, so a caller supplying or reading them
/// all cannot miss one.
pub fn prompt_keys() -> impl Iterator<Item = Words> {
    super::entries()
        .filter(|entry| matches!(entry.kind, Kind::Prompt(_)))
        .map(|entry| Words(entry.key))
}

/// The sub-heading a prompt is drawn under, inside the Prompts group. **In
/// the order Bridge draws them**: the Judge that reads an agent's work, then the
/// agents that are not Sessions. [`super::entries`] lays prompts out in this
/// order whichever table holds them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptSection {
    Judge,
    Scouts,
    Autopilot,
    Helm,
    Retros,
    TestMap,
    Workflows,
}

impl PromptSection {
    /// Every section, in the order Bridge draws them.
    pub const ORDER: [PromptSection; 7] = [
        PromptSection::Judge,
        PromptSection::Scouts,
        PromptSection::Autopilot,
        PromptSection::Helm,
        PromptSection::Retros,
        PromptSection::TestMap,
        PromptSection::Workflows,
    ];

    /// Where the section stands in [`PromptSection::ORDER`]. Exhaustive, so a
    /// new section is placed before it compiles.
    pub const fn position(self) -> usize {
        match self {
            PromptSection::Judge => 0,
            PromptSection::Scouts => 1,
            PromptSection::Autopilot => 2,
            PromptSection::Helm => 3,
            PromptSection::Retros => 4,
            PromptSection::TestMap => 5,
            PromptSection::Workflows => 6,
        }
    }

    /// The sub-heading Bridge draws.
    pub fn title(self) -> &'static str {
        match self {
            PromptSection::Judge => "Judge",
            PromptSection::Scouts => "Scouts",
            PromptSection::Autopilot => "Autopilot",
            PromptSection::Helm => "Helm",
            PromptSection::Retros => "Retros",
            PromptSection::TestMap => "Test map",
            PromptSection::Workflows => "Workflows",
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
        tip: None,
    }
}

pub(super) const PROMPTS: &[Entry] = &[
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
        PromptSection::Autopilot,
        PROMPT_AUTOPILOT_SENTENCE,
        "Reading the sentence",
        "What Autopilot's reader is asked when a person types the one sentence that starts it. {sentence} is the sentence, {now} the time in UTC and {local_now} the person's own clock. Its answer is read by field name, so keep the field names.",
        &["sentence", "now", "local_now"],
    ),
    prompt(
        PromptSection::Autopilot,
        PROMPT_AUTOPILOT_PICKER,
        "Choosing the next issue",
        "What Autopilot's picker is asked for each pick. {instruction} is the person's instruction, {issues} the open issues nothing is working on and {workflows} the workflows the repository holds. Its answer is read by field name, so keep the field names.",
        &["instruction", "issues", "workflows"],
    ),
    prompt(
        PromptSection::Retros,
        PROMPT_RETRO_SESSION,
        "The retro of a job",
        "What the retro of a job is asked. {name} is the job's name, {record} its record, and {where_fixes_land} and {how_to_write} are the two prompts below.",
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
        PromptSection::Retros,
        PROMPT_LOGBOOK_DISTIL,
        "What a job learnt",
        "What the model is asked when a job ends or hands on, to say what the Logbook learns from it. {record} is the job's record since it was last read and {entries} what the Logbook holds now. Its answer is read by field name, so keep the field names.",
        &["record", "entries"],
    ),
    prompt(
        PromptSection::Retros,
        PROMPT_TEACHING_READ,
        "What a person taught",
        "What the model is asked when a person presses Teach on words, a command, a notice, a task, a Check, a pull request or a place in Bridge and says what it is or what to change. {source} names what was pressed on, {may} lists the parts that may be proposed for it, the closed list of conditions a notice rule may test, and the tokens and layout regions a Bridge source may change, {words} is the text pressed on, {message} the message it came from, {said} what the person wrote and {reach} where it applies. Its answer is read by field name, so keep the field names.",
        &["source", "may", "words", "message", "said", "reach"],
    ),
    prompt(
        PromptSection::Helm,
        PROMPT_HELM,
        "Helm",
        "The note appended to the system prompt of a repository's Helm Job, where Helm can act. It says what Helm is and where it is docked; the job otherwise is a Job's. {manifest} and {folder} name the repository and {unasked} the calls on a Studio Helm may make without being asked.",
        &["manifest", "folder", "unasked"],
    ),
    prompt(
        PromptSection::Helm,
        PROMPT_HELM_READ_ONLY,
        "Helm, read only",
        "The note appended to the system prompt of a repository's Helm Job, where Helm only reads. {manifest} and {folder} name the repository.",
        &["manifest", "folder"],
    ),
    prompt(
        PromptSection::Helm,
        PROMPT_HELM_ROUTE,
        "Routing a thought",
        "What the model is asked when a person types into Helm's message box, to decide whether it is a question for Helm or a thought for running work. {candidates} is the live Jobs as a list and {message} the person's words. Its answer is read by field name, so keep the field names.",
        &["candidates", "message"],
    ),
    prompt(PromptSection::Helm, PROMPT_HELM_VOICE, "Voice", "The block Machine Voice is given in. {voice} is the setting.", &["voice"]),
    prompt(
        PromptSection::TestMap,
        PROMPT_TEST_MAP_READING,
        "Reading a file",
        "What the model is asked when main moves and a changed file no collector can measure is read for the tests that exercise it. {file} is the file's path, {text} its text, {tests} the test files it may choose from, {bytes} how much of the text it is shown and {most} how many test files it is offered. Its answer is read as one path a line, so keep that.",
        &["file", "text", "tests", "bytes", "most"],
    ),
    prompt(
        PromptSection::Workflows,
        PROMPT_WORKFLOW_CHANGE,
        "Changing a workflow",
        "What the model is asked when the owner types a change to a workflow, or Armada drafts new words for a step that keeps drifting. {definition} is the workflow's file as it stands, {request} what is wanted of it and {attempt} an earlier draft with what was wrong with it, empty the first time. Its answer is read as the whole new definition, so keep that.",
        &["definition", "request", "attempt"],
    ),
];
