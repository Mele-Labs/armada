//! `runs:`, the ways a repository's app is started in development — apart
//! from [`serving`](super::serving), which reads each server one run lists.
//!
//! **A run is a name for several servers started together.** An app is often a
//! backend and a frontend, and a repository can have more than one way to run
//! (the app, its component gallery). Each server is still a `commands.<name>`
//! with `serve`, on its own `ports:` entry, so a run adds no way to start a
//! process: it only says which ones go up as one.
//!
//! **Resolved against the servers, never the Commands**, as `walk` is: a name
//! that is a Command is refused with what it is, because a run holds what it
//! starts and a Command exits.

use std::collections::{BTreeMap, BTreeSet};

use serde_yaml_ng::Value;

use super::{texts, Command};
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys read inside `runs.<name>`.
const RUN_KEYS: &[&str] = &["label", "commands"];

/// One way to run the app: the servers that go up together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppRun {
    pub(super) label: Option<String>,
    pub(super) servers: Vec<String>,
}

impl AppRun {
    /// What a person is shown for it. **`None` shows the run's name.**
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The servers it starts, in the order the file wrote them. Each is a name
    /// [`Manifest::server`](super::Manifest::server) answers.
    pub fn servers(&self) -> &[String] {
        &self.servers
    }
}

/// The runs by name, and the order the file wrote them in.
pub(super) fn read(
    top: &mut Table<'_>,
    servers: &BTreeSet<String>,
    commands: &BTreeMap<String, Command>,
    out: &mut Vec<Refusal>,
) -> (BTreeMap<String, AppRun>, Vec<String>) {
    let mut built = BTreeMap::new();
    let mut written = Vec::new();
    let Some(value) = top.optional("runs") else {
        return (built, written);
    };
    let Some(table) = Table::open("runs", value, out) else {
        return (built, written);
    };
    for (name, item) in table.into_entries() {
        if let Some(run) = entry(&format!("runs.{name}"), item, servers, commands, out) {
            if built.insert(name.clone(), run).is_none() {
                written.push(name);
            }
        }
    }
    (built, written)
}

fn entry(
    at: &str,
    value: &Value,
    servers: &BTreeSet<String>,
    commands: &BTreeMap<String, Command>,
    out: &mut Vec<Refusal>,
) -> Option<AppRun> {
    let mut table = Table::open(at, value, out)?;
    let label = table
        .optional("label")
        .and_then(|value| yaml::text(&table.at("label"), value, out));
    let listed = table
        .required("commands", out)
        .and_then(|value| yaml::list(&table.at("commands"), value, out))
        .map(|items| texts(items, out));
    table.close(RUN_KEYS, out);
    let mut kept: Vec<String> = Vec::new();
    for (key, name) in listed? {
        if let Some(first_at) = kept.iter().position(|had| *had == name) {
            out.push(Refusal::new(key, Fault::RunListsTwice { first_at }));
        } else if servers.contains(&name) {
            kept.push(name);
        } else if commands.contains_key(&name) {
            out.push(Refusal::new(key, Fault::RunStartsACommand { value: name }));
        } else {
            out.push(Refusal::new(
                key,
                Fault::NotADeclaredServer {
                    value: name,
                    declared: servers.iter().cloned().collect(),
                },
            ));
        }
    }
    Some(AppRun { label, servers: kept })
}
