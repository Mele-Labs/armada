//! One proposal, while the Job proposer is still reading it.
//!
//! **[`JudgeInFlight`] one step earlier, and the step is the difference.** A
//! Judge call rides on a step of a Job; a proposal rides on no step, so it
//! hangs off nothing and needs [`ProposalId`] of its own.
//!
//! **A proposal is an interval with a Job on the Board, from protocol 19.0.**
//! `domain/job-statuses.toml` declares `proposing` for it, so a dispatched
//! request is a row from the moment it is sent rather than nothing until the
//! call answers. Nothing in Fleet creates that Job yet — the status reads
//! `in_code = "Not yet"` — and none of the messages here changed for it: this
//! is still what a client watches the *call* through, and the Job's own row is
//! what it comes back to. Which means [`ProposalId`] stays the subject of a
//! stop, rather than a [`JobId`](crate::ids::JobId) taking it over.
//!
//! **This one ticks and that one does not.** `JudgeInFlight` carries `since`
//! and a budget on the grounds that a surface subtracts for itself, which is
//! right for a gate nobody is watching. Here a person is being asked whether to
//! keep waiting, and "thinking, and here is how much" and "never reached the
//! vendor" are opposite decisions an elapsed count draws identically.
//!
//! What that costs, and the three bounds that hold it, are stated on
//! `proposal.moved` in `crates/ipc/operations.toml`.
//!
//! [`JudgeInFlight`]: crate::JudgeInFlight

use serde::{Deserialize, Serialize};

use crate::enums::Urgency;
use crate::ids::{Instant, ProposalId, WorkflowId};

/// One Job proposer call, while it is still out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalInFlight {
    /// What a stop names. See [`ProposalId`].
    pub proposal_id: ProposalId,
    /// The caller's own token, echoed back unchanged.
    ///
    /// **A correlation token, and deliberately not an id Armada minted.** A
    /// client sends a request and then has to recognise the events about it,
    /// and it cannot key on [`proposal_id`](ProposalInFlight::proposal_id)
    /// because Fleet mints that after the request arrives — the client learns
    /// it from this message. Matching on the request's text instead would match
    /// the wrong call the moment two people dispatch the same words.
    ///
    /// Absent where the caller sent none, which is every caller that is not
    /// watching: Helm proposes without a surface, and has nothing to correlate.
    /// **Fleet neither reads it nor stores it** — it is opaque, echoed, and
    /// gone when the call is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_ref: Option<String>,
    /// Which model is reading the request. The dial that decides what the wait
    /// costs and how long it is likely to be, and a `String` for
    /// [`JudgeInFlight::model`](crate::JudgeInFlight::model)'s reason.
    pub model: String,
    /// When the call went out. Every surface subtracts for itself, exactly as
    /// it does for a Judge call.
    pub since: Instant,
    /// How long the call may take before Fleet gives up on it, in milliseconds.
    ///
    /// **What makes the wait mean something.** A surface drawing ninety seconds
    /// against nothing can only say "this is slow"; drawing it against the
    /// ceiling can say how much of the decision is left, which is the
    /// difference between "still going" and "nearly out of time".
    pub budget_ms: u64,
    /// How far the call has got.
    pub reached: ProposalReach,
    /// The harness's own running estimate of how much the model has thought,
    /// where it has said.
    ///
    /// **Cumulative within this call, and an estimate.** It counts up and is
    /// never added across calls, and the harness calls it an estimate — which
    /// is why every surface renders it as an approximation rather than as a
    /// billed figure. Absent before the model starts thinking, and on a model
    /// that does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_tokens: Option<u64>,
    /// How much of the answer has arrived, in characters.
    ///
    /// Characters rather than tokens because that is what the stream carries at
    /// this point.
    ///
    /// **A count and never the text.** That much is unchanged: the raw answer
    /// does not cross this seam, and [`settled`](ProposalInFlight::settled) is
    /// fields rather than a transcript.
    ///
    /// **What it used to say, and why it said it:** *what the proposer decided
    /// arrives as the Jobs it minted, and a channel carrying the answer as it
    /// was written would be a second, earlier, worse copy of that.* **That was
    /// correct when it was written.** Nothing existed until the answer landed,
    /// so anything read early would have been a rival to the real thing, and
    /// this count was deliberately the whole of what a surface could say about
    /// the answer.
    ///
    /// **30 Sep 2026 changed the premise, not the reasoning.** A dispatched
    /// request is a Job from the press — *a dispatched request is a job*, in the
    /// decisions register — so
    /// there is a row from the moment Dispatch is pressed. A field read off the
    /// answer is not a second copy of that row; it is that row becoming more
    /// complete, and `settled` is how it crosses. **The objection still holds
    /// for the text itself**, which is why this field is still a count and why
    /// nothing beside it carries a transcript.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_characters: Option<u64>,
    /// What the proposer has decided so far.
    ///
    /// **Absent until something has.** A call that has not begun writing has
    /// settled nothing, and an empty object would make a client tell "no field
    /// yet" from "no reading yet" for no difference. Absent also on every Fleet
    /// older than 19.1.
    ///
    /// See [`ProposalSettled`] for what is in it and what is deliberately not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<ProposalSettled>,
}

/// What the proposer has decided, while it is still writing the rest.
///
/// **Four fields, in the owner's order, and the order is his reasoning rather
/// than a layout** (30 Sep 2026): the workflow decides the Job's shape, the
/// title is what makes the row recognisable, done-when is the goal, and the
/// settings are the part he can still change.
/// *A proposal fills in as it is written*, 30 Sep 2026, in the decisions
/// register, carries the option he turned down — four smaller calls, one fact
/// each — and the cost he took for one call read as it arrives.
///
/// # Fields that are settled, never a transcript
///
/// `crates/fleet/src/proposing.rs` is the one reader of the answer's prefix, and
/// a field appears here only once its own line has ended. What that buys is that
/// nothing on this seam carries half a sentence: a client either has a title or
/// has none, and never `Say which of the two was giv`.
///
/// **No `scope`, no `because`, no `after` and no raw text.** `scope` is the brief
/// a Drone is handed, `because` is entry zero's rationale, `after` is the plan's
/// own shape — each reaches a client as the Job it belongs to, once the call has
/// answered. These four are the four a person watching has somewhere to put.
///
/// # The head Job only
///
/// A request that becomes several Jobs is several rows. What is being watched is
/// the row the press made, so this is the first block of the answer and never a
/// shape for rows nobody is looking at yet.
///
/// # A field may land and the call may then die
///
/// `proposing -> escalated` is the ending for a call that faulted, and a Job that
/// got a workflow and no title reaches it with a workflow and no title. **The
/// title's absence is what keeps the row honest**: until one lands, the row's
/// title is the request as it was typed, so a half-read Job still reads as words
/// somebody wrote and never as a blank.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalSettled {
    /// The workflow it chose. **Only ever one this repository holds** — Fleet
    /// checks the name against the catalogue before putting it here, so a
    /// client never draws a workflow that nothing froze and that the call is
    /// about to be refused for naming.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<WorkflowId>,
    /// What the Job is called.
    ///
    /// **This is the field that changes a row under a reader.** Before it lands
    /// the row's title is the request as it was typed; when it lands the row
    /// says something else. That is what was asked for, and a client drawing it
    /// owes the person the fact that it moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// What the Job is held to, one line each, in the order they arrived.
    ///
    /// **This one fills in within itself**, because the answer writes the line
    /// again per criterion rather than one line that grows. Empty is a request
    /// that named none, which is the ordinary case and is what the Done when
    /// card already draws a sentence for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done_when: Vec<String>,
    /// The settings it decided. Absent until the line has ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<ProposalSettings>,
}

/// The settings the proposer answered.
///
/// **A type rather than fields on [`ProposalSettled`] directly**, because the
/// fourth thing a person watches fill is *the settings* — so the fourth field is
/// the settings, and a second one is a field here rather than a fifth field
/// there. Both arrive on one line of the answer, so both settle together.
///
/// **`atomic` is not here and that is a decision rather than an omission.** How
/// the work lands follows from having read the code and this call has read none
/// — the 3 Sep 2026 ruling in `crates/fleet/src/proposal.rs`'s header, which the
/// owner kept on 30 Sep when it was put beside the model. `write_targets` is
/// absent for the same reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalSettings {
    /// How urgent it read the request as being. Absent is a line that named no
    /// urgency, or named one no vocabulary holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urgency: Option<Urgency>,
    /// Which model a Drone on this Job will be spawned as.
    ///
    /// **Only ever one this machine holds** — `list_models`' own set, checked
    /// before this is written, so a client never draws a model nothing can run
    /// and the call is refused for naming one exactly as it is for naming a
    /// workflow nothing holds.
    ///
    /// **Absent is configuration deciding**, which is what `ProposeJob.model`
    /// absent has always meant — never a model the proposer picked as a
    /// default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// How far a proposal has got. **The fact an elapsed count cannot state.**
///
/// # It is not a status and nothing is stored under these names
///
/// `domain/job-statuses.toml` declares what a Job *is*, and every one of those
/// is written down and read back. This is something Fleet is *doing* for as
/// long as it takes and then stops doing — [`crate::JudgeInFlight::look`] makes
/// the same argument for the same reason. Nothing stores one, no transition
/// names one, and a registry row would claim otherwise.
///
/// **`proposing` being a status does not make these five statuses.** That row
/// is the whole interval, stored and transitioned out of; these are how far
/// inside it the call has got, and no edge of the Job machine moves when one
/// changes.
///
/// **A closed set here where `look` is a string**, and the difference is who
/// decides. A look is decided by the four call sites that make one, so a
/// mirrored enum would be a second authority for a list with one. These five
/// are decided by the shape of a model call — start the process, reach the
/// vendor, think, answer, stop — and a client draws a different sentence for
/// each, which is exactly the case a closed set is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalReach {
    /// The process was started and has not said anything yet.
    ///
    /// **The one worth telling apart from every other.** A call sitting here
    /// for ninety seconds never reached the vendor at all, which is a harness
    /// or a credential problem and will not resolve by waiting — the opposite
    /// decision from every other value here.
    Starting,
    /// The harness is up and has announced itself. It has not asked yet.
    Started,
    /// The question is at the vendor. Everything after this is the model's own
    /// time.
    Requesting,
    /// The model is thinking.
    /// [`thinking_tokens`](ProposalInFlight::thinking_tokens) says how much.
    Thinking,
    /// The answer is arriving.
    /// [`answered_characters`](ProposalInFlight::answered_characters) says how
    /// much of it. **The call is nearly over** — a surface may reasonably stop
    /// offering to stop it here, because a stop would throw away work that is
    /// about to land.
    Answering,
}

/// What a person asked Fleet to stop, and the answer.
///
/// **A body rather than a path segment**, unlike every act on a Job. A Job's id
/// is in its route because a Job is a resource with a dozen operations on it; a
/// proposal has exactly one, and it is not a resource — there is no `GET` for
/// it and never will be, because the only thing worth knowing about a proposal
/// in flight arrives as an event and a proposal that has landed is its Jobs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopProposal {
    pub proposal_id: ProposalId,
}

/// What stopping answered.
///
/// **Never a refusal for a proposal that is already gone.** By the time
/// somebody presses stop the call may have just answered, and a 404 there would
/// tell them their press failed when what happened is that they were too late —
/// and the Jobs are on the board either way. So the arms are told apart in the
/// answer rather than by a status code, and both are a success.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalStopped {
    /// Whether a call was actually killed.
    ///
    /// `false` means the proposal had already finished — which is not a
    /// failure, and the sentence a surface draws for it says so.
    pub stopped: bool,
}
