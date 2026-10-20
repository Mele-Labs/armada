//! The headless agent CLI: what an agent is run as. **This is the only file in
//! the workspace that knows how that CLI spells a model or an effort**, and
//! how its transcript lines read as events.
//!
//! **Nothing readable goes in argv**: no prompt text, no task, nothing
//! brokered. `ps` prints a same-uid child's argument list on darwin 27 and does
//! **not** print its environment, measured, so argv is the one channel public
//! by construction. The prompt goes in on stdin as the session's first turn.

use std::convert::Infallible;

use adapter_traits::{AgentHarness, DroneEvent, Effort, FileAct, ToolUse};

use crate::transcript;

/// The program name `crates/config/settings.toml` gives as the default for the
/// AgentHarness binary path: `claude (on PATH)`.
///
/// **The vendor's spelling, and this file is where it is allowed to be.** A
/// composition root that carried its own default would be the boundary having
/// leaked — which is why the composition root calls [`HeadlessAgent::on_path`] rather
/// than passing a string it wrote down.
const ON_PATH: &str = "claude";

/// The models a Job may name, in the order a picker offers them.
///
/// **`crates/config/settings.toml` names none.** Two rows bear on this —
/// `kit-level-allowed-default-models-list`, which is "the set that Default
/// model per Job type and Judge model select from", and
/// `default-model-per-job-type` — and *neither carries a `default` value*,
/// unlike the AgentHarness binary row above it which carries one. So there is
/// no configured roster to read, and the roster has to come from somewhere or
/// the picker has no source.
///
/// It comes from here, for [`ON_PATH`]'s reason: these are the aliases this
/// vendor's CLI accepts on `--model`, so they are the adapter's knowledge and
/// nowhere else's. **That is what is used until the CLI has been asked**, and
/// where it cannot answer: [`HeadlessAgent::ask_models`] is the answer.
///
/// Strongest first, and the order is load-bearing: `session_model` takes the first
/// and `judge_model` the last. `fable` sits second, where the CLI lists it.
const MODELS: &[&str] = &["opus", "fable", "sonnet", "haiku"];

/// The model a Job gets when nothing names one. The middle of [`MODELS`]:
/// capable enough to run a workflow and not the one that empties a budget.
const DEFAULT_MODEL: &str = "sonnet";

/// The headless agent CLI, at a path somebody configured.
///
/// The path is a value rather than a constant so that Doctor can probe a
/// missing binary and a machine can put it somewhere unusual — the alternative
/// is a hardcoded string at a call site, invisible to configuration and to
/// health.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadlessAgent {
    program: String,
}

impl HeadlessAgent {
    /// The CLI at `program`.
    pub fn at(program: impl Into<String>) -> HeadlessAgent {
        HeadlessAgent {
            program: program.into(),
        }
    }

    /// The CLI as installed: the settings default, resolved by the `PATH` the
    /// Drone is given rather than by an absolute path Fleet invented.
    ///
    /// **This is what makes the binary an override rather than a requirement.**
    /// A machine with the CLI installed the ordinary way needs no environment
    /// variable, and a machine that put it somewhere unusual names it — and is
    /// refused before the bind if that name is wrong.
    pub fn on_path() -> HeadlessAgent {
        HeadlessAgent::at(ON_PATH)
    }

    /// The program that will be executed. **The one way to ask what this
    /// harness is, without spelling it** — the name is a vendor's and lives in
    /// this crate, so the composition root probes it through here rather than
    /// carrying a literal it is not allowed to write.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The models a Job may name, for the composer's picker.
    ///
    /// Stated here rather than read from configuration because configuration
    /// states none — see [`MODELS`]. The caller passes it up the seam; nothing
    /// below the adapter boundary learns the spellings.
    pub fn models() -> &'static [&'static str] {
        MODELS
    }

    /// The harness's own name, as a person picks it. **The one place it is
    /// spelled**, for [`program`](Self::program)'s reason.
    pub fn harness_name() -> &'static str {
        "Claude Code"
    }

    /// The model a proposal that names none is given.
    ///
    /// **This is what stops a Job dying at dispatch.** A proposal with no model
    /// used to be accepted, stored and shown on the board, and refused at spawn
    /// with "no model was named". Fleet now fills the value in at creation, and
    /// refuses there when it cannot.
    pub fn default_model() -> &'static str {
        DEFAULT_MODEL
    }
}

impl AgentHarness for HeadlessAgent {
    type Error = Infallible;

    fn read(&self, line: &str) -> Vec<DroneEvent> {
        transcript::read(line)
    }

    /// The path out of an edit call's detail, which [`transcript`] composed
    /// as the path then the size; the whole of it where the row was cut.
    fn edited(&self, event: &DroneEvent) -> Option<String> {
        let DroneEvent::Called { tool, detail, .. } = event else {
            return None;
        };
        if !crate::conversing::wrote_the_checkout(tool) {
            return None;
        }
        let said = detail.whole().unwrap_or(detail.shown());
        let path = crate::conversing::path_written(said).trim();
        (!path.is_empty()).then(|| path.to_string())
    }

    fn used(&self, event: &DroneEvent) -> Option<ToolUse> {
        let DroneEvent::Called { tool, detail, .. } = event else {
            return None;
        };
        crate::conversing::tool_use(tool, detail.whole().unwrap_or(detail.shown()))
    }

    fn edit_act(&self, event: &DroneEvent) -> FileAct {
        match event {
            DroneEvent::Called { tool, .. } if tool == "Write" => FileAct::Write,
            _ => FileAct::Edit,
        }
    }
}

/// `--effort` and its word where one is set: absent is the CLI's own default.
/// **One spelling for every call** — a Drone, a Judge, Helm.
pub(crate) fn thinking(args: &mut Vec<String>, effort: Option<Effort>) {
    if let Some(effort) = effort {
        args.push("--effort".into());
        args.push(
            match effort {
                Effort::Low => "low",
                Effort::Medium => "medium",
                Effort::High => "high",
            }
            .into(),
        );
    }
}
