//! What a Judge call is started as, in a crate that cannot start one.
//!
//! # The same split as the Drone, for a different reason
//!
//! [`ModelClient`] renders an [`Ask`] into a [`JudgeCall`] and Fleet runs it.
//! For a Drone the split makes confinement assertable without spawning; here it
//! also keeps `adapters` free of a process, so the one thing that starts
//! anything stays `fleet`'s.
//!
//! # What a [`JudgeCall`] cannot say
//!
//! There is no worktree, no MCP configuration path and no toolbelt on this
//! type. The one directory it can carry is a [`Reading`]'s: a repository's own
//! checkout, read-only, for a few turns, so a Judge can look up what the
//! repository requires of a change the way a Drone does. Decided 2 Oct 2026.
//! An ask without one is the fence as it was — a call that reaches nothing,
//! because there is no field in which to name anything to reach.
//!
//! **A [`Reading`] names a checkout, never a worktree.** Nothing in this crate
//! can tell the two paths apart, so the rule is the caller's: Fleet builds one
//! from the repository it serves, and a Job's worktree is not a value it holds
//! there. What the argument list makes of it is `adapters`'.
//!
//! The question goes in on stdin for [`crate::harness`]'s reason: argv is
//! world-readable through `ps`, and a criterion quotes the work.

use alloc::string::String;
use alloc::vec::Vec;
use core::num::NonZeroU8;

use crate::harness::{Environment, Model, SpawnConfigRefused};

/// One narrow question, put to one model.
///
/// **One criterion per ask.** A call carrying several questions gets one answer
/// covering all of them, which is the open-ended prompt the narrow-question
/// rule exists to refuse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    model: Model,
    question: String,
    environment: Environment,
    reading: Option<Reading>,
}

/// A repository's checkout a Judge may read, and for how many turns.
///
/// **Read-only is not a field.** There is no way to ask for a write here, so a
/// client renders the read tools or nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    checkout: String,
    turns: NonZeroU8,
}

impl Reading {
    /// `checkout` is the repository's own checkout, absolute. A relative one
    /// would resolve wherever the call happened to start, which is the
    /// mistake [`SpawnConfigRefused::McpConfigPathNotAbsolute`] already names.
    pub fn checkout(checkout: &str, turns: NonZeroU8) -> Result<Reading, SpawnConfigRefused> {
        if !checkout.starts_with('/') {
            return Err(SpawnConfigRefused::CheckoutNotAbsolute {
                given: String::from(checkout),
            });
        }
        let trimmed = checkout.trim_end_matches('/');
        Ok(Reading {
            checkout: String::from(if trimmed.is_empty() { "/" } else { trimmed }),
            turns,
        })
    }

    /// The directory the call starts in and may read under.
    pub fn directory(&self) -> &str {
        &self.checkout
    }

    /// The most turns the call may take, reads and answer together.
    pub fn turns(&self) -> NonZeroU8 {
        self.turns
    }
}

impl Ask {
    /// The only way to make one. An empty question is refused where it is
    /// written rather than answered agreeably by a model.
    ///
    /// The environment is the caller's, not the adapter's: a Judge call is
    /// Fleet's own and authenticates as Fleet, which is the one way it differs
    /// from a Drone's confinement rather than copies it.
    pub fn put(
        model: Model,
        question: &str,
        environment: Environment,
    ) -> Result<Ask, SpawnConfigRefused> {
        if question.trim().is_empty() {
            return Err(SpawnConfigRefused::PromptEmpty);
        }
        Ok(Ask {
            model,
            question: String::from(question),
            environment,
            reading: None,
        })
    }

    /// The same ask, able to read `reading`'s checkout.
    pub fn reading(self, reading: Reading) -> Ask {
        Ask {
            reading: Some(reading),
            ..self
        }
    }

    /// What the call may read, or `None` for a call that reaches nothing.
    pub fn reads(&self) -> Option<&Reading> {
        self.reading.as_ref()
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// The assembled question. **Not an argument** — see this module's comment.
    pub fn question(&self) -> &str {
        &self.question
    }
}

/// A one-shot model call: a process, not yet started.
///
/// **No stdio, and a directory only from the ask.** Where a call that reads
/// nothing runs is chosen by whoever runs it. One that reads starts in its
/// [`Reading`]'s checkout, copied off the ask in [`JudgeCall::rendered`], so an
/// adapter cannot point a Judge anywhere it was not given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeCall {
    program: String,
    args: Vec<String>,
    environment: Environment,
    question: String,
    directory: Option<String>,
}

impl JudgeCall {
    /// Render an ask into a startable process.
    ///
    /// **The question, the environment and the directory come from the ask**,
    /// not from the caller, so an implementation cannot put a different
    /// question on stdin or hand the call something it was not given.
    pub fn rendered(ask: &Ask, program: &str, args: Vec<String>) -> JudgeCall {
        JudgeCall {
            program: String::from(program),
            args,
            environment: ask.environment().clone(),
            question: String::from(ask.question()),
            directory: ask.reads().map(|reading| String::from(reading.directory())),
        }
    }

    /// Marked as running unattended, for the `armada` mod to read.
    pub fn unattended(mut self) -> JudgeCall {
        self.environment = self.environment.unattended();
        self
    }

    /// The checkout the call starts in, where it was given one to read.
    pub fn directory(&self) -> Option<&str> {
        self.directory.as_deref()
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// What goes in on stdin, and then the pipe closes. A Judge takes one turn
    /// because there is nothing left to write to it.
    pub fn question(&self) -> &str {
        &self.question
    }
}

/// A model call that carries no toolset — the Judge, the Job-shape classifier,
/// and generated copy — beyond the read tools a [`Reading`] grants. Distinct
/// from [`crate::AgentHarness`], which carries one.
///
/// # Rendering cannot fail
///
/// There is no associated error type and no `Result`, which is what makes this
/// object-safe: Fleet holds one behind a pointer rather than being generic over
/// it, the way it already holds a clock. Everything an implementation could
/// refuse — a blank model, an empty question — is refused by [`Ask`] before one
/// exists.
pub trait ModelClient {
    /// How this client would ask one question.
    fn render(&self, ask: &Ask) -> JudgeCall;

    /// How this client would ask the same question **so that the asking can be
    /// watched**.
    ///
    /// Same question, same model, same confinement — what differs is that the
    /// call reports on itself while it is out, so a caller can say more than
    /// how long it has been waiting. See [`CallProgress`] for what that buys
    /// and why it is not free.
    ///
    /// **Required rather than defaulted.** A default falling back to
    /// [`render`](ModelClient::render) would make a client that cannot be
    /// watched indistinguishable from one nobody taught to be, and the caller
    /// would draw a progress surface that never moves.
    fn render_watched(&self, ask: &Ask) -> JudgeCall;

    /// Read one line of what a watched call printed.
    ///
    /// **On the trait for the reason [`AgentHarness::read`] is on its own**: a
    /// stream's shape belongs to whoever renders the call, and a caller that
    /// parsed it itself would be Fleet knowing a vendor's output format. It is
    /// the same rule from the other end as `render_watched` — the client says
    /// how to ask, so the client says how to read the answer.
    ///
    /// **Total, and never an error.** A line that does not decode is
    /// [`Heard::Nothing`], because progress is a courtesy and a courtesy must
    /// not be able to fail a call.
    ///
    /// [`AgentHarness::read`]: crate::AgentHarness::read
    fn heard(&self, line: &str) -> Heard;
}

/// What one line of a watched call said.
///
/// **Three arms and not two, because the answer comes down the same pipe.** A
/// call rendered for watching prints its answer as a line of its own stream
/// rather than as plain stdout, so a caller reading only progress would watch a
/// call attentively and never learn what it said.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    /// The call moved.
    Moved(CallProgress),
    /// The model's answer, whole.
    Answer(String),
    /// Nothing the caller cares about. **The ordinary case**: a single call
    /// prints hundreds of frames, and all but a handful say nothing about
    /// either how far it has got or what it decided.
    Nothing,
}

/// How far a watched call has got, as the call's own stream said.
///
/// **The fact a wait could not state.** A model call prints nothing until it is
/// finished, so a person watching one had the elapsed time and nothing else —
/// and "ninety seconds and thinking hard" and "ninety seconds and never reached
/// the API" are the same pixels under an elapsed count. They are what somebody
/// deciding whether to keep waiting is choosing between.
///
/// **A report, never a result.** Nothing here is the answer and no arrangement
/// of these advances anything: a caller that lost every one would get the same
/// outcome a beat later. A courtesy that could change a verdict would be a
/// second authority on what the call said.
///
/// **The ordinary order is not guaranteed.** A model that does not think emits
/// no `Thinking`, so what each one means is that it happened — never that the
/// ones before it did.
///
/// **Not `Copy` since [`Answering`](CallProgress::Answering) carries the frame's
/// own text.** Every other reading here is a handful of bytes, and a `Copy` enum
/// holding a `String` is not expressible. What it costs is that a reporter
/// holding one clones it; neither of the two that do is in a loop that cares.
#[derive(Clone, Debug, PartialEq)]
pub enum CallProgress {
    /// The process started and announced itself. **This is the one that
    /// separates a hung harness from a slow model**: a call with no `Started`
    /// after a minute never got as far as the vendor.
    Started,
    /// The question reached the model. Everything after this is the model's
    /// time rather than the harness's.
    Requesting,
    /// It is thinking, and this is how much of it there has been so far.
    ///
    /// **Cumulative and estimated, both by the stream that says it.** It counts
    /// up within one call and is not added across calls, and it is the
    /// harness's own estimate rather than a billed figure — which is why it
    /// renders as an approximation wherever it is shown.
    Thinking { tokens: u64 },
    /// It is writing the answer: how much of it has arrived, and the frame
    /// that just did.
    ///
    /// Characters rather than tokens because that is what the stream carries
    /// here, and inventing a token count from a character count would be a
    /// second, worse answer to a question the `Thinking` arm already answers
    /// properly.
    ///
    /// **`text` is one frame and never the answer so far.** A reader that wants
    /// the running text joins the frames itself, which keeps this arm the size
    /// of what arrived rather than the size of what has arrived. The whole
    /// answer still comes back as [`Heard::Answer`] and is still the call's
    /// return value.
    ///
    /// **Why it is carried at all.** This arm was a count alone until 30 Sep
    /// 2026, deliberately: `crates/adapters/src/watching.rs` argued that a
    /// progress channel also carrying the answer would be a second copy of it
    /// arriving before the answer, and while nothing existed until the answer
    /// landed that was right. A dispatched request is a Job from the press now,
    /// so a prefix is not a rival to the proposal — it is that Job's own fields
    /// settling one at a time, and `crates/fleet/src/proposing.rs` is the one
    /// reader that turns this into them. **Nothing puts this text on the wire.**
    Answering { characters: u64, text: String },
    /// The call is over, and this is what it cost. **Not the answer** — the
    /// answer is the call's return value, and this says only that there will be
    /// no more progress.
    ///
    /// **The readout `crates/config/settings.toml` says nothing can produce.**
    /// `judge-cost-cap-per-check` is open on the grounds that a call rendered
    /// `--output-format text` emits no result envelope, so a dollar cap there
    /// would be enforced by nothing. A watched call emits one. This carries it
    /// — and **enforcing a cap is still not built**, so that question is
    /// narrowed rather than closed: what was missing was the number, and the
    /// number now exists on the one call that is watched.
    ///
    /// Both are `None` where the stream ended without saying, which is what a
    /// killed call does.
    Ended {
        /// What the vendor billed, in US dollars.
        cost_usd: Option<f64>,
        /// How long the model itself took, in milliseconds. **Not the wait** —
        /// the wait includes starting a process and is the caller's to measure.
        api_ms: Option<u64>,
    },
}
