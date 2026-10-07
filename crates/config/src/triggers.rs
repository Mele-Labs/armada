//! Where a Trigger comes from, and which ones a repository runs.
//! `docs/concepts/trigger.md`.
//!
//! Three places, the most specific winning: Armada's (none), the repository's
//! `.armada/triggers/` and this machine's. Two files with one [`TriggerIdentity`]
//! are one Trigger and the specific copy replaces the other whole, as
//! [`crate::Catalogue`] does for workflows. Text in, not directories: reading the
//! repository's files from `main` only is the caller's.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use core_model::{
    OnTriggerFailure, StepId, Trigger, TriggerIdentity, TriggerResolution, TriggerRuns,
    TriggerSkipped, TriggerSource, TriggerWhen, Ulid, WorkflowId,
};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::manifest::Manifest;
use crate::yaml::{self, Table};

const KEYS: &[&str] = &[
    "name",
    "when",
    "workflow",
    "step",
    "command",
    "skill",
    "on_failure",
];
const FAILURE_KEYS: &[&str] = &["block", "repair"];
const WHENS: &[&str] = &["step_starts", "step_passes", "pr_opened"];
const WHEN_WORDS: &[(&str, TriggerWhen)] = &[
    ("step_starts", TriggerWhen::StepStarts),
    ("step_passes", TriggerWhen::StepPasses),
    ("pr_opened", TriggerWhen::PrOpened),
];

/// One Trigger file as text, and the place it was read from. There is no
/// public way to say a file is Armada's: that set is compiled in.
#[derive(Debug, Clone)]
pub struct Written {
    source: TriggerSource,
    path: PathBuf,
    text: String,
}

impl Written {
    /// A file from the repository's `.armada/triggers/`.
    pub fn in_repository(path: PathBuf, text: String) -> Written {
        Written {
            source: TriggerSource::Repository,
            path,
            text,
        }
    }

    /// A file from this machine's own Triggers. Not Kit's, which travels.
    pub fn on_machine(path: PathBuf, text: String) -> Written {
        Written {
            source: TriggerSource::Machine,
            path,
            text,
        }
    }
}

/// Every Trigger Armada carries. None.
pub fn carried() -> Vec<Written> {
    Vec::new()
}

/// Parse one Trigger file, reporting every fault in it and not the first.
pub fn parse(path: &Path, text: &str) -> Result<Trigger, LoadError> {
    let root: Value = serde_yaml_ng::from_str(text).map_err(|cause| LoadError::NotYaml {
        path: path.to_path_buf(),
        cause,
    })?;
    let mut out = Vec::new();
    match read(&root, &mut out) {
        Some(trigger) if out.is_empty() => Ok(trigger),
        _ => Err(LoadError::Refused {
            path: path.to_path_buf(),
            refusals: out,
        }),
    }
}

fn read(root: &Value, out: &mut Vec<Refusal>) -> Option<Trigger> {
    let mut table = Table::open("", root, out)?;
    let name = table
        .required("name", out)
        .and_then(|v| yaml::text("name", v, out));
    let when = table
        .required("when", out)
        .and_then(|v| yaml::word("when", v, WHEN_WORDS, WHENS, WHENS, out));
    let workflow = table
        .optional("workflow")
        .and_then(|v| yaml::text("workflow", v, out))
        .map(|id| WorkflowId::carried(Ulid::carried(id)));
    let step = table
        .optional("step")
        .and_then(|v| yaml::text("step", v, out))
        .map(StepId::new);
    if when == Some(TriggerWhen::PrOpened) && step.is_some() {
        let wanted = "no step, because `pr_opened` is the delivering step's entry";
        let fault = Fault::WrongType {
            wanted,
            found: "a step",
        };
        out.push(Refusal::new("step", fault));
    }
    let command = table
        .optional("command")
        .and_then(|v| yaml::text("command", v, out));
    let skill = table
        .optional("skill")
        .and_then(|v| yaml::text("skill", v, out));
    let runs = match (table.present("command"), table.present("skill")) {
        (true, true) => {
            let wanted = "a `command` or a `skill`, not both";
            let fault = Fault::WrongType {
                wanted,
                found: "both",
            };
            out.push(Refusal::new("skill", fault));
            None
        }
        (false, false) => {
            let wanted = "a `command` or a `skill`";
            let fault = Fault::WrongType {
                wanted,
                found: "neither",
            };
            out.push(Refusal::new("command", fault));
            None
        }
        _ => command
            .map(TriggerRuns::Command)
            .or_else(|| skill.map(TriggerRuns::Skill)),
    };
    let failure = match table.optional("on_failure") {
        Some(value) => failure(value, out),
        None => Some(OnTriggerFailure::default()),
    };
    table.close(KEYS, out);

    let mut trigger = Trigger::new(name?, when?, runs?).with_failure(failure?);
    if let Some(workflow) = workflow {
        trigger = trigger.in_workflow(workflow);
    }
    if let Some(step) = step {
        trigger = trigger.at_step(step);
    }
    Some(trigger)
}

fn failure(value: &Value, out: &mut Vec<Refusal>) -> Option<OnTriggerFailure> {
    let mut table = Table::open("on_failure", value, out)?;
    let mut flag = |name: &str| match table.optional(name) {
        Some(value) => yaml::flag(&format!("on_failure.{name}"), value, out),
        None => Some(false),
    };
    let block = flag("block");
    let repair = flag("repair");
    table.close(FAILURE_KEYS, out);
    Some(OnTriggerFailure {
        block: block?,
        repair: repair?,
    })
}

/// Why a file is not among the Triggers.
#[derive(Debug)]
pub enum WhyLeftOut {
    /// It will not parse.
    Unparsed(LoadError),
    /// Another file in the same place has the same identity. `also` is every
    /// file after the first.
    Duplicated { also: Vec<PathBuf> },
}

/// A file set aside, and why.
#[derive(Debug)]
pub struct LeftOut {
    source: TriggerSource,
    path: PathBuf,
    why: WhyLeftOut,
}

impl LeftOut {
    pub fn source(&self) -> TriggerSource {
        self.source
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn why(&self) -> &WhyLeftOut {
        &self.why
    }
}

impl fmt::Display for LeftOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a Trigger {} was left out, because ", self.source)?;
        match &self.why {
            WhyLeftOut::Unparsed(why) => write!(f, "{why}"),
            WhyLeftOut::Duplicated { also } => {
                write!(f, "{}", self.path.display())?;
                for path in also {
                    write!(f, " and {}", path.display())?;
                }
                write!(f, " all declare the same one")
            }
        }
    }
}

/// Every Trigger in hand, grouped by identity, and the files already set aside.
#[derive(Debug)]
pub struct Catalogue {
    held: BTreeMap<TriggerIdentity, Vec<(Trigger, TriggerSource)>>,
    left_out: Vec<LeftOut>,
}

impl Catalogue {
    /// Parse every file. The order they arrive in decides nothing. Two in one
    /// place with one identity are both left out; across places it is an
    /// override.
    pub fn of(written: impl IntoIterator<Item = Written>) -> Catalogue {
        let mut placed: BTreeMap<(TriggerSource, TriggerIdentity), Vec<(Trigger, PathBuf)>> =
            BTreeMap::new();
        let mut left_out = Vec::new();
        for one in written {
            match parse(&one.path, &one.text) {
                Ok(trigger) => placed
                    .entry((one.source, trigger.identity()))
                    .or_default()
                    .push((trigger, one.path)),
                Err(why) => left_out.push(LeftOut {
                    source: one.source,
                    path: one.path,
                    why: WhyLeftOut::Unparsed(why),
                }),
            }
        }
        let mut held: BTreeMap<TriggerIdentity, Vec<(Trigger, TriggerSource)>> = BTreeMap::new();
        for ((source, identity), mut triggers) in placed {
            if triggers.len() > 1 {
                let also = triggers.drain(1..).map(|(_, path)| path).collect();
                let (_, path) = triggers.remove(0);
                let why = WhyLeftOut::Duplicated { also };
                left_out.push(LeftOut { source, path, why });
                continue;
            }
            let (trigger, _) = triggers.remove(0);
            held.entry(identity).or_default().push((trigger, source));
        }
        Catalogue { held, left_out }
    }

    /// Pick the most specific copy of each identity and say what it comes to
    /// against `manifest`'s Commands. A copy naming an undeclared Command is
    /// skipped, and does not step down to a less specific one.
    pub fn resolve(self, manifest: &Manifest) -> ResolvedTriggers {
        let triggers = self
            .held
            .into_values()
            .filter_map(|candidates| candidates.into_iter().max_by_key(|(_, source)| *source))
            .map(|(trigger, source)| {
                let resolution = resolution(&trigger, manifest);
                ResolvedTrigger {
                    trigger,
                    source,
                    resolution,
                }
            })
            .collect();
        ResolvedTriggers {
            triggers,
            left_out: self.left_out,
        }
    }
}

fn resolution(trigger: &Trigger, manifest: &Manifest) -> TriggerResolution {
    match trigger.runs() {
        TriggerRuns::Skill(name) => TriggerResolution::Skill { name: name.clone() },
        TriggerRuns::Command(name) => match manifest.command(name) {
            Some(command) => TriggerResolution::Command {
                name: name.clone(),
                asks_first: command.is_destructive(),
            },
            None => TriggerResolution::Skipped(TriggerSkipped::NotInThisRepo {
                command: name.clone(),
            }),
        },
    }
}

/// One Trigger, the place it came from, and what it comes to here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTrigger {
    trigger: Trigger,
    source: TriggerSource,
    resolution: TriggerResolution,
}

impl ResolvedTrigger {
    pub fn trigger(&self) -> &Trigger {
        &self.trigger
    }

    pub fn source(&self) -> TriggerSource {
        self.source
    }

    pub fn resolution(&self) -> &TriggerResolution {
        &self.resolution
    }
}

/// The Triggers a repository runs, and every file set aside on the way.
#[derive(Debug)]
pub struct ResolvedTriggers {
    triggers: Vec<ResolvedTrigger>,
    left_out: Vec<LeftOut>,
}

impl ResolvedTriggers {
    /// One per identity, in identity order. Skipped ones included.
    pub fn triggers(&self) -> &[ResolvedTrigger] {
        &self.triggers
    }

    /// Kept with the reason, so a person is told why a file they wrote is not
    /// running.
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }

    /// Those for `workflow` at `when` on `step`. A `pr_opened` one is asked of
    /// the delivering step.
    pub fn applying<'a>(
        &'a self,
        workflow: &'a WorkflowId,
        when: TriggerWhen,
        step: &'a StepId,
    ) -> impl Iterator<Item = &'a ResolvedTrigger> {
        self.triggers
            .iter()
            .filter(move |one| one.trigger.applies(workflow, when, step))
    }
}
