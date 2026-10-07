//! The same CLI as a Drone, invoked as a call rather than as a session.
//!
//! # It is not a Drone, and the argument list is where that is true
//!
//! No `--input-format stream-json`, so stdin is one prompt and then EOF rather
//! than a session. `--strict-mcp-config` with no `--mcp-config`, so no server
//! at all is reachable — not even the Evidence tool a Drone always holds.
//!
//! # Two fences, chosen by the ask
//!
//! **An ask with no [`Reading`] reaches nothing**: `--max-turns 1`, so the
//! model answers once, and `--tools ""`, so it holds no tool. The Job proposer,
//! generated copy and a link lookup are asked this way.
//!
//! **An ask with one reads the repository**, decided 2 Oct 2026 so that a
//! Judge can learn what a repository requires of a change the way a Drone
//! does — a Judge that could not read refused work CLAUDE.md asked for as
//! *scope expansion*. It starts in the checkout the reading names, a
//! repository's own and never a Job's worktree, and holds:
//!
//! | Argument | What it holds the call to |
//! |---|---|
//! | `--tools Read,Grep,Glob` | the three read tools, and no other in the session |
//! | `--disallowedTools` | every built-in that is not a read, plus `Read(./.armada/**)` |
//! | `--restricted` | reads inside the working directory, measured in spike 017 |
//! | `--max-turns` | the reading's cap, reads and answer together |
//!
//! **`.armada/` is refused** because Fleet's Job worktrees and records are
//! under it: a checkout's `.armada/worktrees/<job>` is the Drone's own work,
//! and a Judge that read it would read how the work was done rather than what
//! the repository requires. Measured on 2 Oct 2026 against 2.1.287: without the
//! rule, Read, Glob and Grep all reached a file under `.armada/worktrees/`
//! although the checkout ignores `.armada/`; with it, all three were refused or
//! came back empty.
//!
//! # `--allowedTools ""` is a floor; `--tools` is the fence
//!
//! The first is a permission allowlist and leaves the toolset standing; a
//! denied use still spends a turn. `--tools` is what the session holds. `#1047`.

use adapter_traits::{Ask, Heard, JudgeCall, ModelClient, Reading};

use crate::harness::HeadlessAgent;
use crate::scouting::{NOT_A_READ, READ_TOOLS};

/// What a reading Judge may not open, however it asks: Fleet's own directory in
/// the checkout, where every Job's worktree and record lives. A permission rule
/// in the CLI's own syntax, relative to the working directory.
const NOT_THE_WORK: &str = "Read(./.armada/**)";

/// The model a request is read by when nothing names one.
///
/// **A constant, where the Judge's is derived.** They are separate dials for
/// the reason `ARMADA_JUDGE_MODEL` is not `ARMADA_MODEL`: a machine that raised
/// one by raising the other would pay the raise on every dispatch as well as on
/// every criterion. `crates/config/settings.toml`'s `job-proposer-model` row
/// still reads `undecided`, so this one is a stand-in stating what the binary
/// takes — the arrangement [`HeadlessAgent::judge_model`] was in until that
/// row was decided.
const PROPOSER_MODEL: &str = "haiku";

/// The model a Job's retro is written on. **Named rather than derived**, and
/// held to the roster by a test: the owner chose the middle tier over the cheap
/// one on 4 Oct 2026, after the cheap one wrote a retro he rejected. It is
/// `crates/config/settings.toml`'s `retro-model` row.
const RETRO_MODEL: &str = "sonnet";

/// The model a judged gaming flag is read a second time on. **Named rather than
/// derived**, and held to the roster by a test: stronger than the first look's,
/// and paid only where a flag was raised.
const SECOND_OPINION_MODEL: &str = "sonnet";

impl HeadlessAgent {
    /// The model a step that names none is judged by: **the cheapest alias on
    /// the roster**, which is what `crates/config/settings.toml`'s
    /// `judge-model` row decided on 28 Aug 2026.
    ///
    /// **Read off the roster rather than written down again.** A constant here
    /// would be a second statement of a value the settings row now owns, and
    /// two statements is how the second one goes stale. The row's peer polarity
    /// is *member of the Kit set*, and an entry of [`HeadlessAgent::models`] is
    /// a member by construction rather than by a check.
    ///
    /// **The last entry, because the roster runs strongest first** — the order
    /// a picker offers, so its end is its cheap end. That is an assumption
    /// about a list this file does not own, which is why a test pins what comes
    /// back to the settings row's value: a reordered roster breaks the build
    /// rather than quietly redeciding a setting.
    pub fn judge_model() -> &'static str {
        match HeadlessAgent::models().last() {
            Some(cheapest) => cheapest,
            // The roster is a non-empty constant, so nothing reaches here. The
            // arm exists so that deriving a value cannot panic at a gate, and
            // it falls back to the model a Job itself would get rather than to
            // a fourth spelling written for the fallback's sake.
            None => HeadlessAgent::default_model(),
        }
    }

    /// The model a dispatch request is read by when nothing names one.
    pub fn proposer_model() -> &'static str {
        PROPOSER_MODEL
    }

    /// The model a Job's retro is written on.
    pub fn retro_model() -> &'static str {
        RETRO_MODEL
    }

    /// The model a judged gaming flag is read a second time on.
    pub fn second_opinion_model() -> &'static str {
        SECOND_OPINION_MODEL
    }
}

impl ModelClient for HeadlessAgent {
    fn render(&self, ask: &Ask) -> JudgeCall {
        JudgeCall::rendered(ask, self.program(), asking(ask, Watched::No)).unattended()
    }

    /// The same call, printing what it is doing while it does it.
    ///
    /// **Every confinement argument is the same one**, which is the property
    /// worth stating: the two renders differ in the output format and in
    /// nothing else, so a watched call is not a call with a longer leash. The
    /// shared builder below is what makes that true by construction rather than
    /// by two lists agreeing.
    fn render_watched(&self, ask: &Ask) -> JudgeCall {
        JudgeCall::rendered(ask, self.program(), asking(ask, Watched::Yes)).unattended()
    }

    /// Read one line of what [`render_watched`](ModelClient::render_watched)
    /// prints. **The other half of that render**, and the reason both are on
    /// one trait: the flag that chooses the format and the reader that
    /// understands it are one decision, and a client that changed the format
    /// without the reader would go quiet rather than fail.
    fn heard(&self, line: &str) -> Heard {
        crate::watching::heard(line)
    }
}

/// Whether the call reports on itself. The one axis the two renders differ on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Watched {
    No,
    Yes,
}

/// The argument list both renders use.
///
/// **One builder, because the confinement is the argument list.** `--tools` is
/// what actually holds a call to its toolset; two lists that happened to agree
/// the day they were written would stop agreeing the first time one of them
/// was edited.
fn asking(ask: &Ask, watched: Watched) -> Vec<String> {
    let mut args: Vec<String> = vec!["-p".into(), "--output-format".into()];
    match watched {
        // The Drone's format, on a call that is not a Drone. It carries the
        // harness's own progress lines, which is the whole reason to pay for
        // it: `--include-partial-messages` is what makes the model's answer
        // arrive as it is written rather than at the end. `crate::watching`
        // reads them, and `--verbose` is what the format requires to emit them.
        Watched::Yes => {
            args.push("stream-json".into());
            args.push("--verbose".into());
            args.push("--include-partial-messages".into());
        }
        // Plain text: the answer on stdout and nothing else. Still the default
        // for the Judge, whose caller has nobody waiting on it — a gate runs
        // without a person watching, and paying for a stream nothing reads
        // would be paying for the surface rather than the verdict.
        Watched::No => args.push("text".into()),
    }
    args.extend([
        "--model".into(),
        ask.model().as_str().into(),
        "--permission-mode".into(),
        "dontAsk".into(),
        // No `--mcp-config` beside it, which is what makes the set empty.
        "--strict-mcp-config".into(),
    ]);
    match ask.reads() {
        None => fenced(&mut args),
        Some(reading) => reading_only(&mut args, reading),
    }
    args
}

/// A call that reaches nothing: one turn and no tool.
fn fenced(args: &mut Vec<String>) {
    args.extend([
        // One turn. A call that can take a second turn is a call that can go
        // looking, and nothing asked this way was given anywhere to look.
        "--max-turns".into(),
        "1".into(),
        "--allowedTools".into(),
        String::new(),
        // The fence `--allowedTools` alone is not: it disables every tool
        // rather than merely denying each use of one. `#1047`.
        "--tools".into(),
        String::new(),
    ]);
}

/// A call that may read its checkout and nothing else. The directory is not an
/// argument: [`JudgeCall::rendered`] copies it off the ask, and Fleet starts
/// the call there.
fn reading_only(args: &mut Vec<String>, reading: &Reading) {
    let mut denied: Vec<&str> = NOT_A_READ.to_vec();
    denied.push(NOT_THE_WORK);
    args.extend([
        "--max-turns".into(),
        reading.turns().to_string(),
        // Reads stay inside the working directory: a bare allow let a call
        // read a sibling directory, and this refused it. Spike 017.
        "--restricted".into(),
        "--tools".into(),
        READ_TOOLS.join(","),
        "--allowedTools".into(),
        READ_TOOLS.join(","),
        // Denied as well as left out of the toolset, since a deny beats an
        // operator's own allow, and `.armada/` with them.
        "--disallowedTools".into(),
        denied.join(","),
    ]);
}
