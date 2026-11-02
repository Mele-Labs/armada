//! `armada job`'s words: an act, then `name=value` fields and at most one JSON body.
//!
//! **No act is named here** but the two that are the CLI's own, `status` and `dispatch`. The rest are
//! whatever the door lists, so an act added to the inventory is callable the day it ships.

use super::{Fault, Verb};

/// What `armada job` was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobAct {
    /// Nothing after it: every act this Job may call.
    Acts,
    /// This Job: its id, repository, mod, door and what is missing.
    Status,
    /// `<act> --help`: what the act does and the fields it takes.
    Help { act: String },
    /// One act, called.
    Call {
        act: String,
        fields: Vec<(String, String)>,
        body: Option<Body>,
    },
}

/// The one JSON argument an act takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// As an argument.
    Inline(String),
    /// `-`: all of stdin.
    Stdin,
}

/// The name the door lists `start_another_session` under, which an agent reads as "dispatch".
pub const DISPATCH: &str = "dispatch";
const DISPATCHES: &str = "start_another_job";
const STATUS: &str = "status";

pub(super) fn read(rest: &[String], faults: &mut Vec<Fault>) -> Option<Verb> {
    let Some((first, words)) = rest.split_first() else {
        return Some(Verb::Job(JobAct::Acts));
    };
    if first == "--help" || first == "-h" {
        return Some(Verb::Job(JobAct::Acts));
    }
    let act = act_named(first);
    if words.iter().any(|word| word == "--help" || word == "-h") {
        return Some(Verb::Job(JobAct::Help { act }));
    }
    if act == STATUS {
        if let Some(given) = words.first() {
            faults.push(Fault::JobStatusTakesNothing { given: given.clone() });
        }
        return Some(Verb::Job(JobAct::Status));
    }
    let mut fields = Vec::new();
    let mut body: Option<Body> = None;
    for word in words {
        let said = match word.as_str() {
            "-" => Some(Body::Stdin),
            json if json.starts_with('{') => Some(Body::Inline(json.to_string())),
            _ => None,
        };
        match (said, field(word)) {
            (Some(said), _) => match body {
                None => body = Some(said),
                Some(_) => faults.push(Fault::JobTwoBodies { given: word.clone() }),
            },
            (None, Some(pair)) => fields.push(pair),
            (None, None) => faults.push(Fault::JobWordUnreadable { given: word.clone() }),
        }
    }
    Some(Verb::Job(JobAct::Call { act, fields, body }))
}

/// The act's name as the door lists it: `-` read as `_`, and `dispatch` as what it starts.
fn act_named(given: &str) -> String {
    match given.replace('-', "_") {
        name if name == DISPATCH => DISPATCHES.to_string(),
        name => name,
    }
}

/// `name=value`, where the name is an identifier. A word that is not one is not a field, so a JSON
/// document with an equals sign inside it is never read as one.
fn field(word: &str) -> Option<(String, String)> {
    let (name, value) = word.split_once('=')?;
    let mut chars = name.chars();
    let opens = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    (opens && chars.all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .then(|| (name.to_string(), value.to_string()))
}
