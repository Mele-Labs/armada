//! One call a Helm session made that the person's own settings do not cover,
//! held open while they answer it in the dock. `#1389`.
//!
//! **A Drone's [`CommandInFlight`](crate::CommandInFlight) one subject over,
//! and deliberately not that type.** Every field of that one is about a Job —
//! `step_id`, `AllowForJob`, a rule written into `armada.yml` on the Job's
//! branch — and a Helm call has no Job. The two sets of answers are different
//! sets and a shared enum would offer a person an answer Fleet cannot take.
//!
//! **What decides is the person's own configuration, not Armada's.** Fleet
//! refuses nothing here and allows nothing here; it carries the question to the
//! dock and carries the answer back inside the call. Spike 19 measured the
//! path, spike 15 measured it first for a Drone.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};

/// What the CLI sends the door when it puts a call to a person — the
/// `--permission-prompt-tool` contract, as spike 15 recorded it.
///
/// **Deserialized here because this is where the bytes are**: `ipc` is one of
/// the two crates that may read JSON, and the tool's arguments arrive as an
/// agent's own object rather than as one of Armada's DTOs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AskingToRun {
    /// The tool the session reached for, in the harness's own spelling.
    pub tool_name: String,
    /// Its whole input, as the session sent it. **Carried back untouched on an
    /// allow** — the CLI runs `updatedInput`, so anything Armada rewrote here
    /// would be Armada editing a person's command behind them.
    #[serde(default)]
    pub input: serde_json::Value,
    /// The harness's id for the call, where it sent one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

impl AskingToRun {
    /// What the call was on, in the words a person would use — a command, a
    /// path, a pattern.
    ///
    /// **`crates/adapters/src/transcript.rs`'s rule, read off a `Value`
    /// instead.** The keys are the tools' own and the order is
    /// first-present-wins, so a tool nothing here names still shows its command
    /// or its path, and none of it is an arm per tool. `None` is a tool whose
    /// arguments this vocabulary has no name for — never an invented line.
    pub fn detail(&self) -> Option<&str> {
        const KEYS: &[&str] = &[
            "command",
            "file_path",
            "path",
            "pattern",
            "url",
            "query",
            "prompt",
            "description",
        ];
        KEYS.iter()
            .find_map(|key| self.input.get(key)?.as_str())
            .filter(|said| !said.trim().is_empty())
    }
}

/// The tool whose call is a question for the person rather than a thing to
/// run. Since 23.71.
pub const ASKS_A_QUESTION: &str = "AskUserQuestion";

/// The four answers a Job's handoff question offers, in order. Fleet starts a successor only when the
/// person chooses [`HANDOFF_YES`]; [`HANDOFF_QUIET`] and [`HANDOFF_REMIND`] are kept on the Job's
/// ledger and change when the Context rule tells it again.
pub const HANDOFF_YES: &str = "Carry on in a new Job";
pub const HANDOFF_STAY: &str = "Stay here";
pub const HANDOFF_QUIET: &str = "Stay, don't ask again this Job";
pub const HANDOFF_REMIND: &str = "Remind me at a higher limit";
/// The question Fleet puts on the card after [`HANDOFF_REMIND`], and the figures it offers, in tokens.
pub const HANDOFF_REMIND_QUESTION: &str = "Ask again at";
pub const HANDOFF_REMIND_AT: [u64; 3] = [450_000, 500_000, 600_000];

/// A figure as the follow-on question labels it: `450k`.
pub fn handoff_figure(tokens: u64) -> String {
    format!("{}k", tokens / 1_000)
}

impl AskingToRun {
    /// Fleet's own follow-on to [`HANDOFF_REMIND`]: one question, [`HANDOFF_REMIND_QUESTION`], with
    /// a figure an option.
    pub fn handoff_remind() -> AskingToRun {
        let options: Vec<serde_json::Value> = HANDOFF_REMIND_AT
            .iter()
            .map(|tokens| serde_json::json!({ "label": handoff_figure(*tokens), "description": "" }))
            .collect();
        AskingToRun {
            tool_name: ASKS_A_QUESTION.to_string(),
            input: serde_json::json!({ "questions": [{
                "question": HANDOFF_REMIND_QUESTION,
                "header": "Context",
                "multiSelect": false,
                "options": options,
            }] }),
            tool_use_id: None,
        }
    }
}

/// One question the agent put to the person, as its tool sent it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskedQuestion {
    /// The question's text, which is also the key its answer goes under.
    pub question: String,
    #[serde(default)]
    pub header: String,
    /// The tool spells it `multiSelect`; the wire spells it `multi_select`.
    #[serde(default, alias = "multiSelect")]
    pub multi_select: bool,
    #[serde(default)]
    pub options: Vec<AskedChoice>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskedChoice {
    pub label: String,
    #[serde(default)]
    pub description: String,
    /// What the agent attached to show while this option is in focus: monospace
    /// text such as a snippet or a list of checks. Empty where it sent none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub preview: String,
}

/// What a person chose for one question: the labels of the options, and where
/// they typed their own words (*Other*), those words as an entry. Since 23.71.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionAnswer {
    pub question: String,
    pub chosen: Vec<String>,
}

#[derive(Deserialize)]
struct Questioned {
    #[serde(default)]
    questions: Vec<AskedQuestion>,
}

impl AskingToRun {
    /// The questions of an [`ASKS_A_QUESTION`] call; empty for any other tool
    /// and for input that carries none.
    pub fn questions(&self) -> Vec<AskedQuestion> {
        if self.tool_name != ASKS_A_QUESTION {
            return Vec::new();
        }
        serde_json::from_value::<Questioned>(self.input.clone())
            .map(|read| read.questions)
            .unwrap_or_default()
    }

    /// The call's input with the person's answers filled in: the tool's
    /// `answers` map, keyed by question text. **A multi-select answer is its
    /// labels joined with `, `**, which is the form the agent CLI reads
    /// (spike 29). Everything else in the input is carried as it came.
    pub fn answered(&self, answers: &[QuestionAnswer]) -> serde_json::Value {
        let mut input = self.input.clone();
        let filled: serde_json::Map<String, serde_json::Value> = answers
            .iter()
            .map(|one| {
                (
                    one.question.clone(),
                    serde_json::Value::String(one.chosen.join(", ")),
                )
            })
            .collect();
        if let Some(object) = input.as_object_mut() {
            object.insert("answers".to_string(), serde_json::Value::Object(filled));
        }
        input
    }
}

/// What the door answers the CLI with. **The whole vocabulary is two words**,
/// and it is the harness's rather than Armada's.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "behavior", rename_all = "lowercase")]
pub enum RunOrNot {
    Allow {
        /// The input as it arrived. See [`AskingToRun::input`].
        #[serde(rename = "updatedInput")]
        updated_input: serde_json::Value,
    },
    Deny {
        /// What the session is shown in place of the call's result. It reaches
        /// the model, so it is written for one.
        message: String,
    },
}

/// What a person may answer about one Helm call.
///
/// **There is no `AllowForJob`** because there is no Job, and no `armada.yml`
/// to write to: Helm runs in the person's own checkout under the person's own
/// settings, so the only place a lasting rule belongs is a settings file of
/// theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelmCallAnswer {
    /// Run it, this once. Nothing is written anywhere, and the same call asks
    /// again next time.
    AllowOnce,
    /// Run it, and write [`HelmCallInFlight::rule`] into the repository's own
    /// personal agent settings — `adapters::PERSONAL_SETTINGS`, which is the
    /// one place that path is spelled — so the CLI allows it without asking,
    /// in Helm and in the person's terminal alike.
    ///
    /// **The only thing in Armada that writes a person's settings**, and it
    /// does so on this answer and no other.
    AllowAndRemember,
    /// Do not run it. The session is told, and goes on without it.
    Refuse,
    /// Run it, and let every session reach the folder the answer names, and everything under it,
    /// with no card. **Armada's folder list, never the agent CLI's rule**: the rule matches a command,
    /// and the guard that drew the card runs before the CLI's settings are read. Offered only on
    /// a call that names [`HelmCallInFlight::reaches`].
    AlwaysAllow,
    /// Do not run it, and refuse every session the folder the answer names, with no card.
    AlwaysBlock,
    /// Run it, and let this session reach the folder the answer names with no card until it
    /// ends. **Held in Fleet's memory and nowhere else**: nothing is written to the folder list or
    /// to settings, and a folder the owner blocked stays blocked.
    AllowForSession,
    /// Run it, and let this session's calls of the kind the answer names run with no card until it
    /// ends, narrowed to the answer's folder where one is given. **Held in Fleet's memory and nowhere
    /// else**: a folder the owner blocked stays blocked, and a removal or a push is still read as one.
    AllowLikeThisForSession,
}

/// Whether a call past a session's slots reads what it names, writes to it, or runs something the
/// guard cannot class as either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReachesTo {
    Read,
    Write,
    /// **Neither known to read nor known to write** (the owner, 30 Oct 2026): a read-only line
    /// asked as a write was closed as an edit he did not want.
    Run,
}

/// The kind of a shell call: its program, and for an interpreter the script it runs. Wide enough
/// for a later call to match, where the whole line would match only itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallKind {
    /// The program as the segment names it, `node`.
    pub program: String,
    /// The script an interpreter runs, `drive.mjs`. Left out for a program alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

/// One call a Helm session is waiting on a person to answer, right now.
///
/// **It is not a status.** No Job moves, nothing is queued, and the wait ends
/// without either — the same argument
/// [`CommandInFlight`](crate::CommandInFlight) makes for a Drone. What is
/// waiting is one process, inside one tool call, for the length of one reply.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmCallInFlight {
    /// Fleet's own id for this ask. **What an answer names**, so an answer
    /// from a window left open across one is a refusal rather than a
    /// coincidence.
    pub call: String,
    /// The repository whose Helm session asked. A person sees several.
    pub manifest_id: ManifestId,
    /// When Fleet was asked, by Fleet's clock.
    pub asked_at: Instant,
    /// The tool reached for, in the harness's own spelling.
    pub tool: String,
    /// The command, or the argument, bounded to one line. **Empty is a tool
    /// whose argument this vocabulary has no name for**, never an invented one.
    pub detail: String,
    /// Whether [`detail`](HelmCallInFlight::detail) is less than what was sent.
    pub truncated: bool,
    /// How many characters the argument had before anything was cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
    /// The argument as the session sent it, **set only where `detail` was cut**: a person allowing
    /// a call reads all of it, and one that fits already travels as `detail`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whole: Option<String>,
    /// **The rule that would have to allow it**, in the CLI's own settings
    /// spelling — `Bash(gh issue list:*)`, `Edit`. What
    /// [`HelmCallAnswer::AllowAndRemember`] writes, and what a person reads
    /// before choosing it.
    pub rule: String,
    /// What a person may answer, in the order a surface draws them. An answer
    /// that is not here is refused.
    pub offers: Vec<HelmCallAnswer>,
    /// How long Fleet will hold the call open, in seconds, from `asked_at`. A
    /// surface says it rather than deriving it: the bound is Fleet's.
    pub holding_for_seconds: u64,
    /// The questions, where the call is the agent asking the person
    /// ([`ASKS_A_QUESTION`]). **Empty for every other call.** Since 23.71.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<AskedQuestion>,
    /// The path past the session's slots that the guard asked about, absolute, and what a folder
    /// for [`HelmCallAnswer::AlwaysAllow`] or [`HelmCallAnswer::AlwaysBlock`] must contain. Left out
    /// for a call that reaches nowhere.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaches: Option<String>,
    /// Whether the call reads or writes past the slots, for a card the guard drew over a reach.
    /// Set where `reaches` is, and also where the path could not be read as one. Left out for any
    /// other call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaches_to: Option<ReachesTo>,
    /// The program and script of the Bash segment whose reach the card is about, and what
    /// [`HelmCallAnswer::AllowLikeThisForSession`] matches. Set only on a Bash reach card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<CallKind>,
}

/// A person's answer to one Helm call. The request half of `answer_helm_call`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerHelmCall {
    /// [`HelmCallInFlight::call`]. An id naming nothing waiting is a 409.
    pub call: String,
    /// One of that call's offers. Anything else is a 409.
    pub answer: HelmCallAnswer,
    /// Why, in the person's own words, carried to the session inside the
    /// refusal. **Only a refusal reads it** — `AnswerCommand::note`'s rule and
    /// its reason: the reason is in mind at the moment of refusing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// One entry per question, for a call that asked some. Since 23.71.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<QuestionAnswer>,
    /// The folder an "always" or a session answer is for: [`HelmCallInFlight::reaches`] or a
    /// folder above it. Read for those three answers, and one without it is refused; a "like this"
    /// answer may carry one to narrow its kind to that folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// The width a "like this" answer allows: the card's own [`HelmCallInFlight::kind`], or the same
    /// with no script. Read for that answer only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<CallKind>,
}

/// `helm.asking_to_run`: a call is waiting on a person. **Absent from
/// [`HelmCallsWaiting`] is the answer having landed**, which is how
/// `job.command_waiting` says the same thing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmAskingToRun {
    pub waiting: HelmCallInFlight,
}

/// `helm.call_answered`: what became of one ask, published whoever ended it —
/// a person, or the bound running out.
///
/// **Helm's act as its own event type**, the rule
/// `studio.helm_acted` already follows. A person who finds a command was run
/// reads this to see that they allowed it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmCallAnswered {
    pub call: String,
    pub manifest_id: ManifestId,
    /// The tool and the one-line detail again, so the record reads on its own.
    pub tool: String,
    pub detail: String,
    pub rule: String,
    pub settled: HelmCallSettled,
    pub at: Instant,
}

/// How one ask ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelmCallSettled {
    /// Nobody was asked. Fleet read the call as none of the three a person is
    /// put in front of — destructive, pushing code to a shared space, writing
    /// off this machine — and it ran. `#1525`.
    ///
    /// **Published for a call no card was ever drawn for**, which is why the
    /// record matters more here than anywhere else in this enum: it is the only
    /// account of what Helm did unasked, and the thing that catches a rule drawn
    /// too wide.
    RanUnasked,
    /// A person allowed it, this once.
    AllowedOnce,
    /// A person allowed it and Fleet wrote the rule into their settings.
    AllowedAndRemembered,
    /// A person allowed it and the rule could not be written. **The call still
    /// ran**: the answer was theirs and a settings file that would not open is
    /// not a reason to refuse them.
    AllowedButNotRemembered,
    /// A person refused it.
    Refused,
    /// Nobody answered inside the bound, so Fleet answered for them with a
    /// refusal. **Never an allow** — silence is not consent.
    Unanswered,
    /// The session went before the answer did. Nothing was run.
    SessionGone,
}

/// What `list_helm_calls` answers: every ask waiting on this person right now,
/// across every repository Fleet serves.
///
/// **Read on a resync and never polled.** The events above carry each ask as it
/// arrives; this is what a Bridge that started mid-wait reads once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmCallsWaiting {
    pub waiting: Vec<HelmCallInFlight>,
}
