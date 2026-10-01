//! What the Job proposer is asked, and how its answer reads. **Pure** —
//! `proposal` makes the call, the way `judging` runs what
//! `verification::Brief` assembles.
//!
//! # Nothing here assigns a workflow by default
//!
//! [`Proposal`] has no arm meaning "the usual one". A call that chooses none,
//! and one that names a workflow this Fleet does not hold, both answer
//! [`Unresolved`] — the request is refused and comes back unchanged. A nearest
//! fit would not be a guess a person could correct later: the definition is
//! frozen into the Job at creation and becomes the yardstick it is judged
//! against.
//!
//! # A call that could not be made is not that refusal
//!
//! `Err` and `Ok(Unresolved)` are different answers and reach a person as
//! different statuses. The network, the quota and the timeout say nothing about
//! the request.
//!
//! # It is not asked which files the work touches
//!
//! That is the first step's, answered by a Drone that has read the code and
//! declared its scope through the scope tool. A proposal guessing paths would
//! be a second source for it, settled worse and earlier.
//! `docs/concepts/job-proposer.md`.

use std::collections::BTreeMap;
use std::fmt;

use config::ResolvedWorkflow;
use core_model::{Ulid, Urgency, WorkflowId};
use verification::field;

use crate::judging::CallFailed;

/// The block an answer owes per Job, and the one word that declines.
///
/// The last four paragraphs are load-bearing and none is decoration. The
/// first is the field order, which is a contract rather than a layout — see
/// [`Settled`]; the second stops one Job becoming three, which is the failure
/// a proposer that *can* split work invents; the third is what makes a member
/// of a split briefable on its own part, since that line is the only thing its
/// Drone is given; the fourth stops a list of paths being answered from.
///
/// **The first four lines are in the owner's order and it is not arbitrary**
/// (30 Sep 2026): the workflow decides the Job's shape, the title is what makes
/// the row recognisable, done-when is the goal, and the settings are the part he
/// can still change. *A proposal fills in as it is written*, 30 Sep 2026, in
/// the decisions register.
const ANSWER_FORMAT: &str = "\
Answer with nothing but the block below, once for each Job the work needs.

    job: <its number, counting from 1>
    workflow: <the id, spelled exactly as it appears above>
    title: <what to call this Job, in the words the request used>
    done_when: <one thing that has to be true before this Job is finished, in \
one line. Write the line again for each one. Leave it out where the request \
names none>
    settings: urgency=<normal, or incident where something is broken for \
people using the software right now>, model=<one of the models listed above, \
spelled exactly. Leave the pair out to let this machine's configuration choose>
    scope: <what this Job is to do, and none of what the others are. Leave \
the line out where you write one Job>
    because: <why that workflow, in one line>
    after: <the job numbers that must finish first, comma separated. Leave \
the line out where none do>

Write the lines in the order above, and finish each line before starting the \
next. Somebody is watching this answer arrive and each line fills a place on \
their screen as it lands, so a line written out of order fills the wrong place \
and a line revised further down moves under them while they read it.

If no workflow above fits the request:

    workflow: none
    because: <what the request asks for that no workflow above covers>

Answer `none` rather than the nearest fit. The workflow named here is frozen \
into the Job and becomes the standard its work is held to, so a near miss is \
not something anybody can correct afterwards.

Write one Job unless the work genuinely cannot land as one change. A split \
that could have landed together is three reviews where one was needed.

Where you write several, `scope` is the whole of what that Job's worker is \
told. It is not shown the rest of the request and it is not shown the other \
Jobs, so anything above that it needs has to be inside its own `scope` line — \
in the requester's own words, on one line, and carrying none of what another \
Job is for.

Do not work out which files are involved. That is the first step's, and it is \
answered there by reading the code rather than guessed at here.";

/// One Job a reading proposed, before Fleet has minted anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedJob {
    pub workflow_id: WorkflowId,
    pub title: String,
    /// **What this Job's Drone is told, and the whole of it.**
    ///
    /// One Job's is the request as the person wrote it; nothing was divided,
    /// so its part is all of it. A member of a split gets its own `scope` line
    /// and nothing else — not the rest of the request, and not its siblings.
    ///
    /// It read the other way until 9 Sep 2026, when a request naming a bug and
    /// an addition became two Jobs and the first one fixed both: every member
    /// carried the whole request, so the split lived in the titles and nowhere
    /// a Drone reads. `read` is where this is settled, because that is where
    /// the plan's size is known and a member without a part can still be
    /// refused.
    pub brief: String,
    /// Why this workflow. **Entry zero's rationale**, and the only durable
    /// trace the call ever ran — nothing else on the record says a proposal
    /// happened.
    pub because: Option<String>,
    /// The one-based positions of the Jobs that must finish first. **Always
    /// earlier than this one**, which is what makes a plan creatable in order.
    pub after: Vec<usize>,
    /// What this Job is held to, one line each, in the order it wrote them.
    ///
    /// **Empty is a request that named none**, which is every request that
    /// describes work without saying when it is finished — and it is what the
    /// Done when card has always drawn as *nothing was read out of a request or
    /// an issue*. Each line reaches the Job as an `AcceptanceCriterion` whose
    /// source is the Judge: these are prose, and prose is what the Judge reads.
    pub done_when: Vec<String>,
    /// How urgent the request says the work is.
    ///
    /// A fact about the request itself — something is broken for people right
    /// now, or it is not.
    pub urgency: Urgency,
    /// Which model a Drone on this Job is spawned as.
    ///
    /// **`None` is configuration's**, and it is what a call that names no model
    /// reaches — never a model this call picked as a default. The owner's
    /// decision of 30 Sep 2026, taken over the argument that a model choosing
    /// which model runs the work is the dial every later call's cost hangs off:
    /// it picks, and it picks only from what this machine holds.
    ///
    /// **`atomic` is not the pair of this and is deliberately absent.**
    /// `crate::proposal`'s own header carries the 3 Sep 2026 ruling: how the
    /// work lands follows from having read the code, and this call has read
    /// none. Put beside this decision on 30 Sep, the owner kept it.
    pub model: Option<String>,
}

/// What one call proposed. **No arm of this means "the usual one".**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Proposal {
    /// The Jobs the request became, in dependency order. Never empty.
    Resolved(Vec<ProposedJob>),
    /// No workflow resolved, and the request is refused at dispatch.
    Unresolved(Unresolved),
}

/// Why no workflow resolved. **Both arms end the same way for the person** —
/// the request comes back — and they are two values because the sentence a
/// person reads is different.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unresolved {
    /// It read the request and none of the workflows covers it.
    NoneFits { because: Option<String> },
    /// It named a workflow this repository does not hold. Refused rather than
    /// nearest-matched: a name nothing holds is not evidence about which one
    /// was meant.
    NotHeld { named: String },
}

impl fmt::Display for Unresolved {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unresolved::NoneFits { because: Some(why) } => write!(
                out,
                "no workflow this repository holds fits the request: {why}"
            ),
            Unresolved::NoneFits { because: None } => {
                out.write_str("no workflow this repository holds fits the request")
            }
            Unresolved::NotHeld { named } => write!(
                out,
                "the proposal named `{named}`, which this repository does not hold"
            ),
        }
    }
}

/// Why no proposal was made at all. **Never an assignment**, and never the
/// refusal above: a request nothing could be read about is not a request that
/// was read and declined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotProposed {
    /// The call itself — the program, the network, the quota, the budget.
    Call(CallFailed),
    /// It answered, and the answer has no `workflow:` line in it.
    NamesNoWorkflow,
    /// A Job in the plan was given no name. Fleet writes none of its own: a
    /// title is the requester's words, and inventing one invents them.
    NamesNoTitle { at: usize },
    /// A Job waits on one that is not before it, or on one that is not in the
    /// plan at all. **Neither is creatable**: an edge points at a minted id, so
    /// a plan is created in order or not at all.
    OutOfOrder { at: usize, after: usize },
    /// A member of a split named no part of the work. **Refused rather than
    /// given the whole request**, which is what it used to get: a Drone handed
    /// the undivided brief does the other Jobs' work as well as its own, and
    /// nothing downstream can tell that apart from the work it was for.
    ///
    /// Only ever a plan of several. One Job's part is the request.
    NamesNoScope { at: usize },
    /// Asked once more after one of the four readings above, and the second
    /// answer could not be read either. **Never [`NotProposed::Call`]** — the
    /// model answered both times; what came back could not be turned into a
    /// plan. Both raw replies are kept here, and also written by
    /// `crate::kept_reply` to a file `kept_at` names — `crate::refusing` puts
    /// only that path on the wire, never the reply text. `#831`.
    Unreadable {
        second: Box<NotProposed>,
        first_reply: String,
        second_reply: String,
        /// Where both replies were written, relative to this repository's own
        /// records. `None` where the write itself did not happen — a
        /// directory that would not open, or a disk that refused — on
        /// `crate::kept_reply::kept`'s own rule that a failed write is not a
        /// reason to fail the refusal it is for.
        kept_at: Option<String>,
    },
}

impl fmt::Display for NotProposed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotProposed::Call(why) => write!(out, "{why}"),
            NotProposed::NamesNoWorkflow => out.write_str(
                "the answer names no workflow at all, so nothing was proposed either way",
            ),
            NotProposed::NamesNoTitle { at } => write!(
                out,
                "job {at} was given no name, and Fleet does not write one of its own"
            ),
            NotProposed::OutOfOrder { at, after } => write!(
                out,
                "job {at} waits on job {after}, which is not before it in the plan"
            ),
            NotProposed::NamesNoScope { at } => write!(
                out,
                "the answer splits the request and job {at} says no part of it is its own"
            ),
            NotProposed::Unreadable { second, .. } => write!(
                out,
                "asked once more, and the second answer could not be read either: {second}"
            ),
        }
    }
}

impl std::error::Error for NotProposed {}

/// What one call is asked, assembled.
///
/// **Held rather than passed as a bare string**, for [`verification::Brief`]'s
/// reason: what the proposer is told is built in one place and asserted on
/// without a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief {
    question: String,
    /// The request as it was handed in, kept because [`read`](Brief::read) is
    /// what settles each Job's own brief and one Job's brief is this.
    request: String,
}

impl Brief {
    /// Assemble the question.
    ///
    /// **Three inputs and no fourth.** There is no parameter for the Manifest,
    /// the repository, the Board or the Jobs already running — every extra token
    /// is money on a call that fires on every dispatch, and a proposer that
    /// could read the repository is a Drone at many times the price. `models` is
    /// the third because the settings line names one, and a list is what stops
    /// it naming a model nothing can run.
    pub fn about(
        request: &str,
        workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>,
        models: &[String],
    ) -> Brief {
        let mut question = String::new();
        question.push_str(
            // **Not "what it will write".** It used to say so, and the answer
            // format below tells the model the opposite in its last line. A
            // proposer asked for paths it cannot see answers with paths it
            // guessed, and `write_targets` is null at the gate precisely so
            // nothing downstream reads a guess as a declaration.
            "You are deciding what a piece of work is: which workflow it runs \
             under, and whether it is one Job or several. Answer only the \
             question at the end.\n\n",
        );
        question.push_str("The request, as the person wrote it:\n\n");
        question.push_str(request);
        question.push_str("\n\nThe workflows this repository holds, and the steps each runs:\n\n");
        if workflows.is_empty() {
            question.push_str("  (this repository holds none)\n");
        }
        for workflow in workflows.values() {
            question.push_str(&format!(
                "  {} — {}\n",
                workflow.id().as_str(),
                workflow.name()
            ));
            // **What it is for, where the definition says, and in the words a
            // requester would use.** The steps below tell one workflow from
            // its neighbours; they do not say whether a request is this kind
            // of work at all. A milestone asked for by link was declined on
            // 3 Sep 2026 while `epic` sat on this list, because nothing in
            // `Plan the wave` is a word the request used. #424.
            //
            // Absent writes no line, so a definition that declares nothing is
            // offered exactly as it was before the key existed.
            if let Some(asked) = workflow.for_requests() {
                question.push_str(&format!("    for: {asked}\n"));
            }
            // The steps are how a workflow is told apart from its five
            // neighbours. A name alone separates `bug` from `revert` and does
            // not separate `feature` from `refactor`.
            let steps: Vec<&str> = workflow.steps().iter().map(|step| step.label()).collect();
            question.push_str(&format!("    {}\n", steps.join(" -> ")));
        }
        // **The models this machine actually holds, so the settings line can
        // name one rather than invent one.** The owner chose on 30 Sep 2026 to
        // let this call pick the model, over the argument that a model choosing
        // which model runs the work is a dial deciding every later cost — and
        // the guard he took with it is that it picks from the list. A name that
        // is not here is refused exactly as a workflow id that is not held is.
        question.push_str("\nThe models this machine can run a worker on:\n\n");
        if models.is_empty() {
            question.push_str("  (this machine has named none, so leave `model` out)\n");
        }
        for model in models {
            question.push_str(&format!("  {model}\n"));
        }
        question.push('\n');
        question.push_str(ANSWER_FORMAT);
        Brief {
            question,
            request: request.to_string(),
        }
    }

    /// The text that goes to the model, exactly as it goes.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Read one answer back, against the workflows this Fleet actually holds.
    ///
    /// **The catalogue is a parameter and not a convenience.** It is what makes
    /// a workflow nothing holds unrepresentable in the `Ok` half: there is no
    /// input to this that produces a `Resolved` naming an id the map does not
    /// have, none that produces one with a blank title, none that produces a
    /// plan whose edges do not point backwards, and none that produces a
    /// member of a split holding the undivided request as its brief.
    pub fn read(
        &self,
        answer: &str,
        held: &BTreeMap<WorkflowId, ResolvedWorkflow>,
        models: &[String],
    ) -> Result<Proposal, NotProposed> {
        let blocks = blocks(answer);
        // Declining is one answer about the whole request rather than one Job's
        // line, so it is read off the first block and ends the reading.
        let Some(first) = blocks.first() else {
            return Err(NotProposed::NamesNoWorkflow);
        };
        if declines(first) {
            return Ok(Proposal::Unresolved(Unresolved::NoneFits {
                because: field(first, "because"),
            }));
        }
        let mut jobs: Vec<(Option<String>, ProposedJob)> = Vec::with_capacity(blocks.len());
        for (position, block) in blocks.iter().enumerate() {
            let at = position + 1;
            let Some(named) = field(block, "workflow") else {
                return Err(NotProposed::NamesNoWorkflow);
            };
            let workflow_id = WorkflowId::carried(Ulid::carried(named.clone()));
            if !held.contains_key(&workflow_id) {
                return Ok(Proposal::Unresolved(Unresolved::NotHeld { named }));
            }
            let title = field(block, "title").ok_or(NotProposed::NamesNoTitle { at })?;
            let after = after(block);
            if let Some(&ahead) = after.iter().find(|&&waits| waits >= at || waits == 0) {
                return Err(NotProposed::OutOfOrder { at, after: ahead });
            }
            // **Held rather than resolved here**, because what one member's
            // brief is depends on how many there are and the last block has
            // not been read yet. A single `scope` on a plan of one is dropped
            // below rather than refused: the line was asked to be left out,
            // and a model that wrote one anyway has not said anything the
            // request does not already say better.
            //
            // **`scope_field`, not `field`.** `ANSWER_FORMAT` asks for it on
            // one line, and a reply shaped as a table or a list is the one
            // that does not comply — `#831`. `field` stays what the Judge
            // reads unchanged; only the proposer tolerates the wrap.
            let scope = scope_field(block);
            // **Refused exactly as an unheld workflow is**, and that is the
            // point of reusing the arm rather than minting a second shape of
            // wrongness: a name nothing holds is not evidence about which one
            // was meant, so the request comes back rather than being spawned
            // against a model this machine cannot run.
            let model = named_model(block);
            if let Some(named) = &model {
                if !models.iter().any(|held| held == named) {
                    return Ok(Proposal::Unresolved(Unresolved::NotHeld {
                        named: named.clone(),
                    }));
                }
            }
            jobs.push((
                scope,
                ProposedJob {
                    workflow_id,
                    title,
                    brief: String::new(),
                    because: field(block, "because"),
                    after,
                    done_when: done_when(block),
                    // **Normal where the line is absent or unreadable**, which
                    // is the same value this call sent before it asked at all.
                    // An unspellable urgency is not a reason to refuse a plan:
                    // the word is a setting a person changes at the gate, and
                    // the gate is in front of them either way.
                    urgency: urgency(block).unwrap_or(Urgency::Normal),
                    model,
                },
            ));
        }
        let split = jobs.len() > 1;
        let jobs = jobs
            .into_iter()
            .enumerate()
            .map(|(position, (scope, job))| match split {
                false => Ok(ProposedJob {
                    brief: self.request.clone(),
                    ..job
                }),
                true => match scope {
                    Some(part) => Ok(ProposedJob { brief: part, ..job }),
                    None => Err(NotProposed::NamesNoScope { at: position + 1 }),
                },
            })
            .collect::<Result<Vec<ProposedJob>, NotProposed>>()?;
        Ok(Proposal::Resolved(jobs))
    }
}

/// One block per `job:` line, or the whole answer where it names none.
///
/// The tolerance is one-directional on purpose: an answer for a single Job that
/// skipped the numbering is still one Job, while an answer for several that
/// skipped it is not expressible — there would be nothing for `after` to name.
fn blocks(answer: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in answer.lines() {
        if line.trim().starts_with("job:") {
            found.push(String::new());
        }
        if let Some(current) = found.last_mut() {
            current.push_str(line);
            current.push('\n');
        }
    }
    match found.is_empty() {
        true => vec![answer.to_string()],
        false => found,
    }
}

/// Whether this block declines rather than choosing.
fn declines(block: &str) -> bool {
    field(block, "workflow").is_some_and(|named| named.eq_ignore_ascii_case("none"))
}

/// Every line that opens a field, in the order [`ANSWER_FORMAT`] states them.
/// What [`scope_field`] reads until: the next of these, or the end of the
/// block, and what [`Settled`] reads a field as ended by.
const FIELD_LINES: [&str; 8] = [
    "job:",
    "workflow:",
    "title:",
    "done_when:",
    "settings:",
    "scope:",
    "because:",
    "after:",
];

/// Read `scope`, spanning every line after it up to the next field line or the
/// end of the block, joined with single spaces.
///
/// **Only `scope` calls this.** `workflow`, `title`, `because` and `after`
/// keep [`verification::field`]'s single-line reading — the Judge relies on
/// it unchanged, and none of the four is asked to hold as much of a request's
/// own words as `scope` is. A reply shaped as a table or a list is the one
/// that wraps `scope` onto the lines after it, and this is what a plan of
/// several stops being refused for that alone. `#831`.
fn scope_field(block: &str) -> Option<String> {
    let mut lines = block.lines();
    let mut value = loop {
        let line = lines.next()?;
        if let Some(rest) = line.trim().strip_prefix("scope:") {
            break rest.trim().to_string();
        }
    };
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if FIELD_LINES.iter().any(|known| trimmed.starts_with(known)) {
            break;
        }
        if !value.is_empty() {
            value.push(' ');
        }
        value.push_str(trimmed);
    }
    (!value.is_empty()).then_some(value)
}

/// Every `done_when` line in this block, in the order it wrote them.
///
/// **One line each, and repeats are the shape rather than a mistake** —
/// [`ANSWER_FORMAT`] asks for the line again per criterion, because a person
/// watching the answer arrive gets one more line on the screen each time rather
/// than one line that grows. A blank one is dropped.
fn done_when(block: &str) -> Vec<String> {
    block
        .lines()
        .filter_map(|line| line.trim().strip_prefix("done_when:"))
        .map(str::trim)
        .filter(|said| !said.is_empty())
        .map(str::to_string)
        .collect()
}

/// One `key=value` pair off the `settings` line.
///
/// `key=value`, comma separated, so a second setting is a second pair rather
/// than a second line — which is what keeps the four fields four.
fn setting(block: &str, key: &str) -> Option<String> {
    let said = field(block, "settings")?;
    said.split(',')
        .filter_map(|pair| pair.split_once('='))
        .find(|(named, _)| named.trim().eq_ignore_ascii_case(key))
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// What the `settings` line said about urgency, where it said anything.
fn urgency(block: &str) -> Option<Urgency> {
    Urgency::from_wire(setting(block, "urgency")?.to_ascii_lowercase().as_str())
}

/// What the `settings` line named as the model, as it spelled it.
///
/// **Unchecked here.** Whether this machine holds it is the caller's, because
/// the two callers answer differently: [`Brief::read`] refuses the plan and
/// [`Settled`] simply does not settle the field — which is what the workflow id
/// one line up already does in each of them.
fn named_model(block: &str) -> Option<String> {
    setting(block, "model")
}

/// What the proposer has settled, read off an answer that is still being
/// written.
///
/// # A prefix is the same Job, not a second copy of it
///
/// `crates/ipc/src/proposing.rs` carried the opposite rule until 30 Sep 2026 —
/// a count and never the text, because a channel carrying the answer as it was
/// written would be a second, earlier, worse copy of the Jobs it minted. That
/// was right while nothing existed until the answer landed. A dispatched
/// request is a Job from the press now, so there is a row from the moment
/// Dispatch is pressed and a field read early is that row becoming more
/// complete. The DTO carries the dated correction.
///
/// # A field is settled when its own line has ended
///
/// The format is one field per line, so a `\n` is what says a value will not
/// change. Everything after the last newline is a line still being written and
/// is not read at all — which is what stops a title landing as `Say which of
/// the two was giv`.
///
/// # The head Job only
///
/// A request that becomes several Jobs is several rows, and the extras carry
/// `dispatched_by`. What is being watched is the row the press made, so the
/// reading stops at the second `job:` line rather than proposing a shape for
/// rows nobody is looking at yet.
///
/// # What it never carries
///
/// No `scope`, no `because`, no `after`, and no raw text. `scope` is a brief
/// and a brief is what the Drone is handed; a transcript on the wire is the
/// thing the correction above is careful not to become. Four fields, because
/// four fields are what a person watching has somewhere to put.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    /// The workflow, **only where this repository holds it**. A name nothing
    /// holds ends the call as [`Unresolved::NotHeld`], and drawing it in the
    /// meantime would put a workflow on screen that nothing froze.
    pub workflow: Option<WorkflowId>,
    /// What the Job is called. Until this lands the row's title is the request
    /// as it was typed, and when it lands the title changes under whoever is
    /// reading it.
    pub title: Option<String>,
    /// What the Job is held to, in the order the lines arrived.
    pub done_when: Vec<String>,
    /// How urgent it read the request as being. `None` until the line ends.
    pub urgency: Option<Urgency>,
    /// The model it chose, **only where this machine holds it** — the workflow
    /// field's own rule one line down. `None` is also a call that named none,
    /// which is configuration deciding and not a default this picked.
    pub model: Option<String>,
}

impl Settled {
    /// Read what has settled out of the answer as far as it has been written.
    ///
    /// **Pure, and the whole prefix every time.** It re-reads rather than
    /// folding frame by frame, because a fold would have to hold a
    /// half-written line and decide what to do with it; re-reading a few
    /// hundred characters on a throttled tick costs nothing and cannot drift
    /// from what the finished answer says.
    pub fn of(
        written: &str,
        held: &BTreeMap<WorkflowId, ResolvedWorkflow>,
        models: &[String],
    ) -> Settled {
        // Everything up to and including the last newline. A line with no
        // newline after it is still being written.
        let ended = match written.rfind('\n') {
            Some(at) => &written[..=at],
            None => return Settled::default(),
        };
        let head = head_block(ended);
        let workflow = field(&head, "workflow")
            .map(|named| WorkflowId::carried(Ulid::carried(named)))
            .filter(|named| held.contains_key(named));
        Settled {
            workflow,
            title: field(&head, "title"),
            done_when: done_when(&head),
            urgency: urgency(&head),
            model: named_model(&head).filter(|named| models.iter().any(|held| held == named)),
        }
    }
}

/// The first Job's block of a partly written answer, whether or not it is
/// numbered.
///
/// **[`blocks`]'s tolerance, one block deep.** An answer that skipped `job:`
/// is one Job and the whole prefix is its block; an answer that numbered it
/// ends the block where the second number starts.
fn head_block(ended: &str) -> String {
    let mut out = String::new();
    let mut opened = false;
    for line in ended.lines() {
        if line.trim().starts_with("job:") {
            if opened {
                break;
            }
            opened = true;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The job numbers this block waits on. A word that is not a number is dropped
/// rather than refused — `after: none` is a model saying no.
fn after(block: &str) -> Vec<usize> {
    field(block, "after")
        .map(|line| {
            line.split(',')
                .filter_map(|entry| entry.trim().parse::<usize>().ok())
                .collect()
        })
        .unwrap_or_default()
}
