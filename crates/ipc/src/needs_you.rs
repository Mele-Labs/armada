//! `get_needs_you`: what a Job waits on the person for, grouped once in Fleet so each Bridge draws the
//! same band. `crates/ipc/operations/get_needs_you.toml` is the contract.
//!
//! Every `waiting_for` item is one row of exactly one section. The rule per source is the table in
//! `crates/fleet/src/session_host/needs_you.rs`, and `docs/concepts/session/waiting-on-the-person.md` states it again.

use serde::{Deserialize, Serialize};

use crate::sessions::{SessionId, WaitingOption, WaitingSource};
use crate::BuildSource;

/// A Job's Needs you: ordered sections, and what the band's head reads off them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsYou {
    pub session_id: SessionId,
    /// In the order the band draws them: Questions, To do, To review, Walks, Commands, PRs, then Restart and
    /// Done. A section with no row is left out.
    pub sections: Vec<NeedsYouSection>,
    /// Whether the band has a list to open: a row in Questions, To do, Walks, Commands or PRs. False when only
    /// pages to review, a restart key or a done are owed.
    pub fold: bool,
    /// Whether the band offers the review sheet: more than one page is owed, or a Document is the only
    /// thing owed. False, a lone plan, sketch or draft stays on the thread's card, and `glyphs` leaves
    /// the review marks out. The To review section still lists it.
    pub review_in_band: bool,
    /// The marks on the band's head, in order: each row of Questions, To do, Walks, Commands and PRs, named by
    /// its text, then, where `review_in_band`, each row of To review, named by what it is.
    pub glyphs: Vec<NeedsYouGlyph>,
}

/// One group of rows, under its label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsYouSection {
    pub kind: NeedsYouSectionKind,
    /// The words the band heads it with.
    pub label: String,
    /// In the order of `waiting_for`; To review, oldest first.
    pub rows: Vec<NeedsYouRow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedsYouSectionKind {
    /// Something to answer: a choice, a reply, or a press that goes to a question or a message.
    Questions,
    /// Something the agent asked him to do, with nothing to press but Done: an item it stated that offers no
    /// choices, takes no answer, and runs nothing. `done_waiting` is the press.
    ToDo,
    /// A plan, sketch, draft or Document he reads, then approves or sends back.
    ToReview,
    /// A page to walk.
    Walks,
    /// A command to run, steps to follow, or Fleet's own offer of a restart.
    Commands,
    /// A pull request to open.
    PullRequests,
    /// A restart the agent or a reply asked for: not a list row but the band's own key, pressed folded.
    Restart,
    /// The agent said it is done. Not part of the list the band folds.
    Done,
}

/// One item of `waiting_for`, as the band draws it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsYouRow {
    /// The item's id: what `dismiss_waiting` and `answer_waiting` take.
    pub id: String,
    pub source: WaitingSource,
    pub text: String,
    /// When it began to wait.
    pub since: crate::Instant,
    pub glyph: NeedsYouGlyph,
    pub act: NeedsYouAct,
    pub dismiss: NeedsYouDismiss,
}

/// A mark and the words a tooltip names it by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsYouGlyph {
    pub kind: NeedsYouGlyphKind,
    pub said: String,
}

/// What a mark draws. The Bridge keeps the picture for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedsYouGlyphKind {
    /// A row with choices.
    Question,
    /// A row he answers with words.
    Reply,
    /// A question Fleet read out of the agent's reply.
    ReadFromReply,
    Walk,
    /// A press that goes to the question.
    Answer,
    PullRequest,
    Run,
    /// A press that goes to the message box.
    Message,
    Steps,
    /// An item with no act.
    Waiting,
    /// A Job this one's agent started, held on its start card.
    LinkedJob,
    /// What the Operator would do.
    Operator,
    /// Something he does, then says is done.
    Todo,
    Plan,
    Sketch,
    Draft,
    Document,
    Restart,
    Done,
}

/// What pressing the row does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NeedsYouAct {
    /// Open the plan for approval.
    OpenPlan,
    /// Open the sketch under this title.
    OpenSketch { title: String },
    /// Open the Document at this path.
    OpenDocument { path: String },
    /// Open the draft card of this thread row.
    OpenDraft { row: String },
    /// Open the page in the walk window.
    Walk { url: String },
    /// Answer by a choice or, with `other`, by words.
    Answer {
        options: Vec<WaitingOption>,
        /// Whether words are taken besides the options.
        other: bool,
    },
    /// Answer by words in the box under the row.
    Reply,
    /// Copy the command, or open it in a terminal.
    Run { command: String },
    /// Open the procedure.
    Steps {
        steps: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
    },
    /// Open the pull request: its `#number`, as the item's act names it.
    ApprovePr { target: String },
    /// Open the Job whose id this is.
    OpenJob { id: String },
    /// Go to the Job's message box.
    Message,
    /// Switch the build, as the Fleet panel's key does. `label` is the key's words (`Refresh preview`).
    Restart { label: String, build: BuildSource, adopt: bool, command: String },
    /// He did it: `done_waiting`, with a note if he has one (`2 remain`). The Job is sent `Done: <text>` and the
    /// note. `steps` is the procedure, where the item carries one; empty otherwise.
    Done {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        steps: Vec<String>,
    },
    /// Nothing to press.
    None,
}

/// What putting the row away is, and what `dismiss_waiting` does for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedsYouDismiss {
    /// Answered, never dismissed: Fleet refuses `dismiss_waiting`, or the card answers it.
    None,
    /// One press. Fleet tells the agent where the item was its own or a walk, a sketch, a Document, a
    /// plan.
    Plain,
    /// One press named Skip. The agent is told nothing.
    Skip,
    /// A press that may carry a reason, which the agent is sent.
    Reason,
    /// A sentence read from a reply, marked as no question: `dismiss_waiting` with `misread`.
    NotAQuestion,
}
