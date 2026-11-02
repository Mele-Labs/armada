//! The helper: one fresh call for one decision, which names a way to answer and says whether it is
//! sure. **Pure but for [`decided`]**, and holding no place: it reads what the decision is and how
//! this repository's earlier decisions were answered, and it ends.
//!
//! **Sure or leave, and nothing between.** A call that names no way, says it is not sure, or is
//! unreadable leaves the item with the person. A kind that is destructive never reaches it
//! (`super::destructive`), and the way it names is read for that again before it is acted on.

use std::fmt;

use adapter_traits::Ask;
use config::settings::PROMPT_AUTOPILOT_DECIDER;
use ipc::AutopilotDecision;
use store::KeptDecision;

use super::sentence::line;
use super::Asking;
use crate::judging::{said, CallFailed};
use crate::prompts::Prompts;

/// The question, with `{instruction}`, `{kind}`, `{decision}` and `{earlier}` to fill. As it ships:
/// `prompts.autopilotDecider` is the setting that overrides it.
pub(crate) const QUESTION: &str = "\
You answer one decision for a repository while the person who owns it is away. Answer only the \
block at the end.

What the person asked Autopilot to do, in their words:

    {instruction}

The kind of decision:

    {kind}

What is being decided:

{decision}
How this repository's earlier decisions were answered, newest first. A correction is what the \
person wanted instead:

{earlier}
Choose one way to answer, or none. Then say whether you are sure. Be sure only where the \
instruction and the earlier answers plainly point to that way. Where the decision may need \
something the person has not said, where the ways differ in what they cost, or where you cannot \
tell, choose none: it waits for the person.

Answer with nothing but these three lines:

    choice: <the number of one way to answer, or none>
    sure: <yes or no>
    because: <why, in one line the person will read in the morning>";

/// How many earlier decisions of a repository the helper reads.
pub(crate) const EARLIER: u32 = 20;

/// One decision, as the helper is shown it. `ways` are the answers it may name, in order; a kind
/// with one yes to give has one, which says what yes does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Deciding {
    pub kind: AutopilotDecision,
    pub subject: String,
    /// What the subject is about, where it has more to read than its words.
    pub detail: Option<String>,
    pub ways: Vec<String>,
}

/// What the helper made of one decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reading {
    /// Sure: the way it named, by index into [`Deciding::ways`], and why.
    Answer { way: usize, because: String },
    /// Not sure, or no way named: the item waits.
    Leave { because: String },
}

/// Why nothing was decided. **Never an answer**: the item waits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NotDecided {
    Call(CallFailed),
    Unreadable { why: String, answer: String },
}

impl fmt::Display for NotDecided {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotDecided::Call(failed) => write!(out, "the helper did not answer: {failed}"),
            NotDecided::Unreadable { why, .. } => write!(out, "the helper's answer was refused: {why}"),
        }
    }
}

fn described(kind: AutopilotDecision) -> &'static str {
    match kind {
        AutopilotDecision::Recommended => "a question the agent asked, with one answer marked recommended",
        AutopilotDecision::Open => "a question the agent asked, with answers and none marked recommended",
        AutopilotDecision::Plan => "a plan the agent recorded, to approve",
        AutopilotDecision::Draft => "a draft the agent wrote, to send as written",
        AutopilotDecision::Walk => "a change the person would see, to approve after a walk",
        AutopilotDecision::Land => "a pull request whose checks passed, to land",
        AutopilotDecision::Unsure => "an issue the picker was unsure of, to start or leave",
        AutopilotDecision::Red => "a red main, to fix",
        AutopilotDecision::Reads => "a tool that only reads, to allow once",
        AutopilotDecision::Writes => "a tool that edits files or runs a command, to allow once",
    }
}

fn earlier_lines(earlier: &[KeptDecision]) -> String {
    let mut out = String::new();
    for one in earlier {
        out.push_str(&format!("  {} ({})\n    chose: {}\n    because: {}\n", one.asked, one.kind, one.chose, one.because));
        if let Some(said) = &one.corrected {
            out.push_str(&format!("    corrected: {said}\n"));
        }
    }
    if out.is_empty() {
        out.push_str("  none yet\n");
    }
    out
}

/// The question one decision is put as, in the words of `prompts.autopilotDecider`.
pub(crate) fn question(prompts: &Prompts, instruction: &str, deciding: &Deciding, earlier: &[KeptDecision]) -> String {
    let mut decision = format!("  {}\n", deciding.subject);
    if let Some(detail) = &deciding.detail {
        for text in detail.lines() {
            decision.push_str(&format!("    {text}\n"));
        }
    }
    decision.push_str("The ways to answer:\n");
    for (at, way) in deciding.ways.iter().enumerate() {
        decision.push_str(&format!("  {}. {way}\n", at + 1));
    }
    prompts.fill(
        PROMPT_AUTOPILOT_DECIDER,
        &[
            ("instruction", instruction.trim()),
            ("kind", described(deciding.kind)),
            ("decision", &decision),
            ("earlier", &earlier_lines(earlier)),
        ],
    )
}

/// Ask once, and read the answer against the ways offered.
pub(crate) async fn decided(
    instruction: &str,
    deciding: &Deciding,
    earlier: &[KeptDecision],
    asking: &Asking,
) -> Result<Reading, NotDecided> {
    let question = question(&asking.prompts, instruction, deciding, earlier);
    let ask = Ask::put(asking.model.clone(), &question, asking.environment.clone())
        .map_err(|_| NotDecided::Call(CallFailed::NothingToAsk))?;
    let answer = said(asking.client.as_ref(), &ask, asking.budget).await.map_err(NotDecided::Call)?;
    read_decision(&answer, deciding.ways.len())
}

/// Read an answer. A way is named by its number among `ways`; `none` or an unsure answer leaves it.
pub(crate) fn read_decision(answer: &str, ways: usize) -> Result<Reading, NotDecided> {
    let unreadable = |why: String| NotDecided::Unreadable { why, answer: answer.to_string() };
    let because = line(answer, "because")
        .filter(|said| !said.is_empty())
        .unwrap_or_else(|| "it gave no reason".to_string());
    let named = line(answer, "choice").ok_or_else(|| unreadable("it named no choice".to_string()))?;
    if named.eq_ignore_ascii_case("none") {
        return Ok(Reading::Leave { because });
    }
    let way = named
        .parse::<usize>()
        .ok()
        .filter(|at| (1..=ways).contains(at))
        .ok_or_else(|| unreadable(format!("choice {named} was not one of the ways it was offered")))?;
    match line(answer, "sure").map(|said| said.to_ascii_lowercase()).as_deref() {
        Some("yes") => Ok(Reading::Answer { way: way - 1, because }),
        Some("no") => Ok(Reading::Leave { because }),
        _ => Err(unreadable("it did not say whether it was sure".to_string())),
    }
}
