//! The Do of a rule file: each item is one act, named by its kind, and takes the keys of that kind
//! and no others. The kinds are the words an autonomy default is set against.

use std::collections::BTreeMap;

use core_model::rule::{Act, ActKind, ForgeWrite, TrackerWrite, Words};
use core_model::StateId;
use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};
use crate::lifecycle::id_of;
use crate::yaml::{self, Table};

use super::moment::target_of;
use super::{the_one, yes};

const KINDS: &[&str] = &[
    "tracker_write",
    "forge_write",
    "draft",
    "start_job",
    "start_watch",
    "tell_agent",
    "run_command",
    "run_agent",
];
const KIND_WANTED: &str = "one of `tracker_write`, `forge_write`, `draft`, `start_job`, `start_watch`, `tell_agent`, `run_command` or `run_agent`";

const TRACKER: &[&str] = &["assign", "move", "close", "on"];
const TRACKER_WRITES: &[&str] = &["assign", "move", "close"];
const TRACKER_WANTED: &str = "one of `assign`, `move` or `close`";
const FORGE: &[&str] = &["request_reviewers", "mark_ready", "merge", "auto_merge", "on"];
const FORGE_WRITES: &[&str] = &["request_reviewers", "mark_ready", "merge", "auto_merge"];
const FORGE_WANTED: &str = "one of `request_reviewers`, `mark_ready`, `merge` or `auto_merge`";
const DRAFT: &[&str] = &["template", "on"];
const START_JOB: &[&str] = &["prompt", "workflow"];
const START_WATCH: &[&str] = &["recipe", "values"];
const TELL: &[&str] = &["message", "wake"];
const TELL_WANTED: &str = "one of `message` or `wake`";
const COMMAND: &[&str] = &["command"];
const AGENT: &[&str] = &["skill", "drone"];
const AGENT_WANTED: &str = "one of `skill` or `drone`";

pub(super) fn act_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<Act> {
    let mut item = Table::open(at, value, out)?;
    let one = the_one(&mut item, at, KINDS, KIND_WANTED, out);
    item.close(KINDS, out);
    let (key, value) = one?;
    let at = format!("{at}.{key}");
    let mut table = Table::open(&at, value, out)?;
    match ActKind::from_wire(key)? {
        ActKind::TrackerWrite => tracker(table, &at, out),
        ActKind::ForgeWrite => forge(table, &at, out),
        ActKind::Draft => {
            let template = required_words(&mut table, "template", out);
            let target = target(&mut table, out);
            table.close(DRAFT, out);
            Some(Act::Draft { template: template?, target: target? })
        }
        ActKind::StartJob => {
            let prompt = required_words(&mut table, "prompt", out);
            let workflow = match table.optional("workflow") {
                None => Some(None),
                Some(value) => words(&table.at("workflow"), value, out).map(Some),
            };
            table.close(START_JOB, out);
            Some(Act::StartJob { workflow: workflow?, prompt: prompt? })
        }
        ActKind::StartWatch => {
            let recipe = required_words(&mut table, "recipe", out);
            let values = match table.optional("values") {
                None => Some(BTreeMap::new()),
                Some(value) => values_of(&table.at("values"), value, out),
            };
            table.close(START_WATCH, out);
            Some(Act::StartWatch { recipe: recipe?, values: values? })
        }
        ActKind::TellAgent => {
            let one = the_one(&mut table, &at, TELL, TELL_WANTED, out);
            table.close(TELL, out);
            let (key, value) = one?;
            let at = format!("{at}.{key}");
            match key {
                "message" => words(&at, value, out).map(|message| Act::Tell { message }),
                _ => yes(&at, value, out).map(|()| Act::Wake),
            }
        }
        ActKind::RunCommand => {
            let command = required_words(&mut table, "command", out);
            table.close(COMMAND, out);
            Some(Act::RunCommand { command: command? })
        }
        ActKind::RunAgent => {
            let one = the_one(&mut table, &at, AGENT, AGENT_WANTED, out);
            table.close(AGENT, out);
            let (key, value) = one?;
            let written = words(&format!("{at}.{key}"), value, out)?;
            Some(match key {
                "skill" => Act::RunSkill { skill: written },
                _ => Act::RunDrone { prompt: written },
            })
        }
    }
}

fn tracker(mut table: Table<'_>, at: &str, out: &mut Vec<Refusal>) -> Option<Act> {
    let one = the_one(&mut table, at, TRACKER_WRITES, TRACKER_WANTED, out);
    let target = target(&mut table, out);
    table.close(TRACKER, out);
    let (key, value) = one?;
    let at = format!("{at}.{key}");
    let write = match key {
        "assign" => words(&at, value, out).map(|to| TrackerWrite::Assign { to }),
        "move" => id_of(&at, value, StateId::new, out).map(|to| TrackerWrite::Move { to }),
        _ => yes(&at, value, out).map(|()| TrackerWrite::Close),
    };
    Some(Act::Tracker { write: write?, target: target? })
}

fn forge(mut table: Table<'_>, at: &str, out: &mut Vec<Refusal>) -> Option<Act> {
    let one = the_one(&mut table, at, FORGE_WRITES, FORGE_WANTED, out);
    let target = target(&mut table, out);
    table.close(FORGE, out);
    let (key, value) = one?;
    let at = format!("{at}.{key}");
    let write = match key {
        "request_reviewers" => yaml::list(&at, value, out)
            .and_then(|names| super::each(names, |at, name| words(at, name, out)))
            .map(|from| ForgeWrite::RequestReviewers { from }),
        "mark_ready" => yes(&at, value, out).map(|()| ForgeWrite::MarkReady),
        "merge" => yes(&at, value, out).map(|()| ForgeWrite::Merge),
        _ => yes(&at, value, out).map(|()| ForgeWrite::AutoMerge),
    };
    Some(Act::Forge { write: write?, target: target? })
}

/// The `on` an act is done to. Required: a write with nothing to write to says nothing.
fn target(table: &mut Table<'_>, out: &mut Vec<Refusal>) -> Option<core_model::rule::Target> {
    let value = table.required("on", out)?;
    target_of(&table.at("on"), value, out)
}

fn words(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<Words> {
    yaml::text(at, value, out).and_then(|written| Words::new(&written).ok())
}

fn required_words(table: &mut Table<'_>, key: &str, out: &mut Vec<Refusal>) -> Option<Words> {
    let value = table.required(key, out)?;
    words(&table.at(key), value, out)
}

fn values_of(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<BTreeMap<Words, Words>> {
    let entries = Table::open(at, value, out)?.into_entries();
    let read: Vec<Option<(Words, Words)>> = entries
        .iter()
        .map(|(name, value)| {
            let key = Words::new(name).inspect_err(|_| out.push(Refusal::new(at, Fault::Empty))).ok();
            let said = words(&format!("{at}.{name}"), value, out);
            Some((key?, said?))
        })
        .collect();
    read.into_iter().collect::<Option<Vec<_>>>().map(|pairs| pairs.into_iter().collect())
}
