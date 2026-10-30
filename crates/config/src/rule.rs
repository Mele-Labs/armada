//! A lifecycle rule, read off a piece file.
//!
//! One file is one rule: `rule_id` and `name` say which, `scope` says where it applies (a
//! Workspace's id or a repository's; absent is every repository), `when` says the moment, `if`
//! lists conditions that must all hold, and `do` lists the acts. `autonomy` and `on_failure` are
//! optional. **A rule here is a lifecycle rule** (When, If and Do with an autonomy), and not a
//! notice rule of `fleet.toml`, which `ipc::FleetRule` names.
//!
//! ```yaml
//! version: 1
//! rule_id: plan_moves_tickets
//! name: Move the tickets when planning starts
//! when: { step_starts: plan }
//! if:
//!   - any: { held: ticket, in: todo }
//! do:
//!   - tracker_write: { move: in_progress, on: { held: ticket, in: todo } }
//! ```
//!
//! **A condition is one of a closed set, and the file is refused for any other**, naming the key
//! and the set, as Teach refuses an `if` it cannot test. The keys an act takes are closed in the
//! same way. Two layers refuse and each says only what it knows: this file's layer knows the keys
//! and what each holds, and [`core_model::rule::Rule::declared`] knows what a whole rule must be.
//!
//! **Text in, not directories**, so a test drives it with no file.

mod acts;
mod applying;
mod moment;

use std::path::{Path, PathBuf};

use core_model::rule::{Autonomy, MalformedRule, OnFailure, Retries, Rule, RuleId, RuleParts, Then};
use core_model::{LifecycleScope, ScopeId, Title};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::lifecycle::id_of;
use crate::yaml::{self, Table};

pub use applying::{RuleFile, Rules};

const TOP_LEVEL: &[&str] = &["version", "rule_id", "name", "scope", "when", "if", "do", "autonomy", "on_failure"];
const FAILURE_KEYS: &[&str] = &["retries", "then"];
const AUTONOMY: &[(&str, Autonomy)] = &[("acts", Autonomy::Acts), ("asks", Autonomy::Asks), ("off", Autonomy::Off)];
const AUTONOMY_LEGAL: &[&str] = &["acts", "asks", "off"];
const THEN: &[(&str, Then)] = &[("tell_agent", Then::TellAgent), ("ask_owner", Then::AskOwner)];
const THEN_LEGAL: &[&str] = &["tell_agent", "ask_owner"];

/// The one version of the file there is.
const VERSIONS: &[&str] = &["1"];

/// Why a rule file did not load.
#[derive(Debug)]
pub enum RuleError {
    /// It is not YAML, or the file holds a fault of its own.
    Load(LoadError),
    /// Every key is right, and the rule they make is not.
    Malformed { path: PathBuf, why: MalformedRule },
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleError::Load(load) => load.fmt(f),
            RuleError::Malformed { path, why } => write!(f, "{} {why}", path.display()),
        }
    }
}

impl std::error::Error for RuleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RuleError::Load(load) => Some(load),
            RuleError::Malformed { why, .. } => Some(why),
        }
    }
}

/// Read a rule from the text of its file. `path` is only named in a refusal.
pub fn parse(path: &Path, text: &str) -> Result<Rule, RuleError> {
    let root: Value = serde_yaml_ng::from_str(text)
        .map_err(|cause| RuleError::Load(LoadError::NotYaml { path: path.to_path_buf(), cause }))?;
    let mut out = Vec::new();
    let read = read(&root, &mut out);
    let Some(parts) = read.filter(|_| out.is_empty()) else {
        return Err(RuleError::Load(LoadError::Refused { path: path.to_path_buf(), refusals: out }));
    };
    Rule::declared(parts).map_err(|why| RuleError::Malformed { path: path.to_path_buf(), why })
}

fn read(root: &Value, out: &mut Vec<Refusal>) -> Option<RuleParts> {
    let mut top = Table::open("", root, out)?;
    if let Some(version) = top.required("version", out).and_then(|value| yaml::positive("version", value, out)) {
        if version != 1 {
            out.push(Refusal::new("version", Fault::NotYetCarried { value: version.to_string(), carried: VERSIONS }));
        }
    }
    let id = top.required("rule_id", out).and_then(|value| id_of("rule_id", value, RuleId::new, out));
    let name = top
        .required("name", out)
        .and_then(|value| yaml::text("name", value, out))
        .and_then(|written| Title::new(&written).ok());
    let scope = match top.optional("scope") {
        None => Some(LifecycleScope::Everywhere),
        Some(value) => yaml::text("scope", value, out)
            .and_then(|written| ScopeId::new(&written).ok())
            .map(LifecycleScope::Within),
    };
    let when = top.required("when", out).and_then(|value| moment::when_of("when", value, out));
    let conditions = match top.optional("if") {
        None => Some(Vec::new()),
        Some(value) => yaml::list("if", value, out)
            .and_then(|items| each(items, |at, item| moment::condition_of(at, item, out))),
    };
    let does = top
        .required("do", out)
        .and_then(|value| yaml::list("do", value, out))
        .and_then(|items| each(items, |at, item| acts::act_of(at, item, out)));
    let autonomy = match top.optional("autonomy") {
        None => Some(None),
        Some(value) => yaml::word("autonomy", value, AUTONOMY, AUTONOMY_LEGAL, AUTONOMY_LEGAL, out).map(Some),
    };
    let on_failure = match top.optional("on_failure") {
        None => Some(OnFailure::default()),
        Some(value) => failure_of("on_failure", value, out),
    };
    top.close(TOP_LEVEL, out);

    Some(RuleParts {
        id: id?,
        name: name?,
        scope: scope?,
        when: when?,
        conditions: conditions?,
        does: does?,
        autonomy: autonomy?,
        on_failure: on_failure?,
    })
}

/// Every item read, **each one before any is given up on**, so a file with three faults in three
/// items is refused for three.
pub(crate) fn each<T>(items: Vec<(String, &Value)>, mut read: impl FnMut(&str, &Value) -> Option<T>) -> Option<Vec<T>> {
    let read: Vec<Option<T>> = items.iter().map(|(at, item)| read(at, item)).collect();
    read.into_iter().collect()
}

/// How a failure is handled. Either key may be left out for the default's.
fn failure_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<OnFailure> {
    let mut table = Table::open(at, value, out)?;
    let default = OnFailure::default();
    let retries = match table.optional("retries") {
        None => Some(default.retries),
        Some(value) => {
            let key = table.at("retries");
            yaml::counted(&key, value, out).and_then(|times| match Retries::new(times) {
                Ok(retries) => Some(retries),
                Err(_) => {
                    out.push(Refusal::new(
                        key,
                        Fault::WrongType { wanted: "a number of retries, five or fewer", found: "a number over five" },
                    ));
                    None
                }
            })
        }
    };
    let then = match table.optional("then") {
        None => Some(default.then),
        Some(value) => yaml::word(&table.at("then"), value, THEN, THEN_LEGAL, THEN_LEGAL, out),
    };
    table.close(FAILURE_KEYS, out);
    Some(OnFailure { retries: retries?, then: then? })
}

/// The one of `names` a table holds. **Exactly one**: none leaves a thing with nothing to say and
/// two a thing with two answers, and neither is guessed at.
pub(crate) fn the_one<'a>(
    table: &mut Table<'a>,
    at: &str,
    names: &'static [&'static str],
    wanted: &'static str,
    out: &mut Vec<Refusal>,
) -> Option<(&'static str, &'a Value)> {
    let found: Vec<(&'static str, &'a Value)> =
        names.iter().filter_map(|name| table.optional(name).map(|value| (*name, value))).collect();
    match found.as_slice() {
        [one] => Some(*one),
        [] => {
            out.push(Refusal::new(at, Fault::WrongType { wanted, found: "none of them" }));
            None
        }
        _ => {
            out.push(Refusal::new(at, Fault::WrongType { wanted, found: "more than one" }));
            None
        }
    }
}

/// `true`, for a key whose presence is the whole of what it says. `false` is refused, since
/// `close: false` reads as the opposite of what a key that is there means.
pub(crate) fn yes(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<()> {
    match yaml::flag(at, value, out)? {
        true => Some(()),
        false => {
            out.push(Refusal::new(at, Fault::WrongType { wanted: "true, or leave the key out", found: "false" }));
            None
        }
    }
}
