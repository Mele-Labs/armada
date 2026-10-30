//! A lifecycle, read off a piece file.
//!
//! One file is one lifecycle. `lifecycle_id` and `name` say which, `scope` says where it applies
//! (the id of a Workspace, meaning a set of repositories, or of one repository; absent is every
//! repository, which is where the four Armada carries apply), and `states`, `transitions` and
//! `ends` are its shape. A state's `read` names exactly one of four sources and what to look for
//! in it, so a state is a fact a reader can see and never something a Job says.
//!
//! **Nothing here touches the Manifest's workspace**, the monorepo unit `manifest/workspace.rs`
//! reads. The two share a word and no code.
//!
//! **Two layers refuse, and each says only what it knows.** This file's layer knows the keys and
//! what each holds, and refuses as a workflow does: every fault of the file, with its key.
//! [`core_model::Lifecycle::declared`] knows what a whole lifecycle must be (a transition names a
//! state it has, nothing is read two ways, something ends it) and that refusal arrives as
//! [`LifecycleError::Malformed`], once the file has no fault of its own.
//!
//! **Text in, not directories**, so a test drives it with no file. Reading the places a piece is
//! kept, and which of two lifecycles with one id wins, is other work.

use std::path::{Path, PathBuf};

use core_model::{
    FleetFact, ForgeFact, Lifecycle, LifecycleId, LifecycleScope, LifecycleState, LifecycleTransition,
    MalformedLifecycle, NotAnId, ReadingState, ScopeId, StateId, StateRead, Title, TrackerWords,
};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::yaml::{self, Table};

const TOP_LEVEL: &[&str] = &["version", "lifecycle_id", "name", "scope", "states", "transitions", "ends"];
const STATE_KEYS: &[&str] = &["id", "label", "read"];
const SOURCES: &[&str] = &["tracker", "forge", "watch", "fleet"];
const TRANSITION_KEYS: &[&str] = &["from", "to"];

/// The one version of the file there is.
const VERSIONS: &[&str] = &["1"];

const FORGE_CARRIED: &[(&str, ForgeFact)] = &[
    ("draft", ForgeFact::Draft),
    ("ready", ForgeFact::Ready),
    ("reviewers_on", ForgeFact::ReviewersOn),
    ("in_review", ForgeFact::InReview),
    ("approved", ForgeFact::Approved),
    ("merged", ForgeFact::Merged),
    ("closed", ForgeFact::Closed),
];
const FORGE_LEGAL: &[&str] = &["draft", "ready", "reviewers_on", "in_review", "approved", "merged", "closed"];

const WATCH_CARRIED: &[(&str, ReadingState)] = &[
    ("waiting", ReadingState::Waiting),
    ("running", ReadingState::Running),
    ("passed", ReadingState::Passed),
    ("failed", ReadingState::Failed),
];
const WATCH_LEGAL: &[&str] = &["waiting", "running", "passed", "failed"];

const FLEET_LEGAL: &[&str] = &["started", "step_entered:<step id>", "pull_request_opened", "landed", "ended"];

const AN_ID: &str = "an id of lower case letters, digits and underscores that starts with a letter";

/// Why a lifecycle file did not load.
#[derive(Debug)]
pub enum LifecycleError {
    /// It is not YAML, or the file holds a fault of its own.
    Load(LoadError),
    /// Every key is right, and the lifecycle they make is not.
    Malformed { path: PathBuf, why: MalformedLifecycle },
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleError::Load(load) => load.fmt(f),
            LifecycleError::Malformed { path, why } => write!(f, "{} {why}", path.display()),
        }
    }
}

impl std::error::Error for LifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LifecycleError::Load(load) => Some(load),
            LifecycleError::Malformed { why, .. } => Some(why),
        }
    }
}

/// Read a lifecycle from the text of its file. `path` is only named in a refusal.
pub fn parse(path: &Path, text: &str) -> Result<Lifecycle, LifecycleError> {
    let root: Value = serde_yaml_ng::from_str(text)
        .map_err(|cause| LifecycleError::Load(LoadError::NotYaml { path: path.to_path_buf(), cause }))?;
    let mut out = Vec::new();
    let read = read(&root, &mut out);
    let Some(parts) = read.filter(|_| out.is_empty()) else {
        return Err(LifecycleError::Load(LoadError::Refused { path: path.to_path_buf(), refusals: out }));
    };
    Lifecycle::declared(parts.id, parts.name, parts.scope, parts.states, parts.transitions, parts.ends)
        .map_err(|why| LifecycleError::Malformed { path: path.to_path_buf(), why })
}

struct Parts {
    id: LifecycleId,
    name: Title,
    scope: LifecycleScope,
    states: Vec<LifecycleState>,
    transitions: Vec<LifecycleTransition>,
    ends: Vec<StateId>,
}

fn read(root: &Value, out: &mut Vec<Refusal>) -> Option<Parts> {
    let mut top = Table::open("", root, out)?;
    if let Some(version) = top.required("version", out).and_then(|value| yaml::positive("version", value, out)) {
        if version != 1 {
            out.push(Refusal::new("version", Fault::NotYetCarried { value: version.to_string(), carried: VERSIONS }));
        }
    }
    let id = top
        .required("lifecycle_id", out)
        .and_then(|value| id_of("lifecycle_id", value, LifecycleId::new, out));
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
    let states: Option<Vec<LifecycleState>> = top
        .required("states", out)
        .and_then(|value| yaml::list("states", value, out))
        .map(|items| items.iter().filter_map(|(at, item)| state_of(at, item, out)).collect());
    let transitions: Vec<LifecycleTransition> = top
        .optional("transitions")
        .and_then(|value| yaml::list("transitions", value, out))
        .map(|items| items.iter().filter_map(|(at, item)| transition_of(at, item, out)).collect())
        .unwrap_or_default();
    let ends: Option<Vec<StateId>> = top
        .required("ends", out)
        .and_then(|value| yaml::list("ends", value, out))
        .map(|items| items.iter().filter_map(|(at, item)| id_of(at, item, StateId::new, out)).collect());
    top.close(TOP_LEVEL, out);

    Some(Parts { id: id?, name: name?, scope: scope?, states: states?, transitions, ends: ends? })
}

/// An id, or the refusal naming what an id is. **No trimming**: `pr: approved` is two words and a
/// colon, and quietly keeping the first would be a guess.
pub(crate) fn id_of<T>(key: &str, value: &Value, make: fn(&str) -> Result<T, NotAnId>, out: &mut Vec<Refusal>) -> Option<T> {
    let written = yaml::text(key, value, out)?;
    match make(&written) {
        Ok(id) => Some(id),
        Err(_) => {
            out.push(Refusal::new(key, Fault::WrongType { wanted: AN_ID, found: "text that is not one" }));
            None
        }
    }
}

fn state_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<LifecycleState> {
    let mut table = Table::open(at, value, out)?;
    let id = table.required("id", out).and_then(|value| id_of(&table.at("id"), value, StateId::new, out));
    let label = table
        .required("label", out)
        .and_then(|value| yaml::text(&table.at("label"), value, out))
        .and_then(|written| Title::new(&written).ok());
    let read = table.required("read", out).and_then(|value| read_of(&table.at("read"), value, out));
    table.close(STATE_KEYS, out);
    Some(LifecycleState::new(id?, label?, read?))
}

/// What a state is read from. **Exactly one source**: two would be a state with two answers.
fn read_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<StateRead> {
    let mut table = Table::open(at, value, out)?;
    let (tracker, forge, watch, fleet) =
        (table.optional("tracker"), table.optional("forge"), table.optional("watch"), table.optional("fleet"));
    let key = |source: &str| table.at(source);
    let (tracker_at, forge_at, watch_at, fleet_at) = (key("tracker"), key("forge"), key("watch"), key("fleet"));
    table.close(SOURCES, out);

    match (tracker, forge, watch, fleet) {
        (Some(words), None, None, None) => tracker_words(&tracker_at, words, out).map(StateRead::Tracker),
        (None, Some(fact), None, None) => {
            yaml::word(&forge_at, fact, FORGE_CARRIED, FORGE_LEGAL, FORGE_LEGAL, out).map(StateRead::Forge)
        }
        (None, None, Some(state), None) => {
            yaml::word(&watch_at, state, WATCH_CARRIED, WATCH_LEGAL, WATCH_LEGAL, out).map(StateRead::Watch)
        }
        (None, None, None, Some(fact)) => {
            let written = yaml::text(&fleet_at, fact, out)?;
            match FleetFact::from_wire(&written) {
                Some(fact) => Some(StateRead::Fleet(fact)),
                None => {
                    out.push(Refusal::new(fleet_at, Fault::NotInTheSchema { value: written, legal: FLEET_LEGAL }));
                    None
                }
            }
        }
        (None, None, None, None) => {
            out.push(Refusal::new(
                at,
                Fault::WrongType { wanted: "one of `tracker`, `forge`, `watch` or `fleet`", found: "none of them" },
            ));
            None
        }
        _ => {
            out.push(Refusal::new(
                at,
                Fault::WrongType { wanted: "one of `tracker`, `forge`, `watch` or `fleet`", found: "more than one" },
            ));
            None
        }
    }
}

/// The tracker's own words, a list that may be empty: a state nobody has mapped yet reads nothing.
fn tracker_words(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<TrackerWords> {
    let Value::Sequence(items) = value else {
        out.push(Refusal::new(at, Fault::WrongType { wanted: "a list", found: yaml::kind(value) }));
        return None;
    };
    let words: Vec<Option<String>> = items
        .iter()
        .enumerate()
        .map(|(n, item)| yaml::text(&format!("{at}[{n}]"), item, out))
        .collect();
    let words: Vec<String> = words.into_iter().collect::<Option<_>>()?;
    // A blank word is already refused above, at its own index, so a duplicate is what is left.
    match TrackerWords::of(words) {
        Ok(words) => Some(words),
        Err(_) => {
            let found = "a word listed twice";
            out.push(Refusal::new(at, Fault::WrongType { wanted: "a list of different words", found }));
            None
        }
    }
}

fn transition_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<LifecycleTransition> {
    let mut table = Table::open(at, value, out)?;
    let from = table.required("from", out).and_then(|value| id_of(&table.at("from"), value, StateId::new, out));
    let to = table.required("to", out).and_then(|value| id_of(&table.at("to"), value, StateId::new, out));
    table.close(TRANSITION_KEYS, out);
    Some(LifecycleTransition { from: from?, to: to? })
}
