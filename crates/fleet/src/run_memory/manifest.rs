//! Writing what a repository runs into its own `armada.yml`, for the `repository` place.
//!
//! **Every kind is a Command, never a Check.** This is a note of how the repository runs, and a
//! Check gates a pull request: nothing remembered from a Job's habits gates the next one. A dev
//! server is a Command with `serve`, the rest a Command with `run`.
//!
//! **A line the file already declares is never overwritten**, and so is a name a person chose:
//! the fact waits, with why. Only the newest command of a kind is written in a workspace. A
//! workspace without an `armada.yml` of its own is not given one.

use std::collections::BTreeMap;
use std::path::Path;

use config::{amend, CommandEdit, Edit, NewCommand, NewLink, NotAmended};
use store::KeptRun;

/// The Command a kind is written as.
fn name_of(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "tests" => "test",
        "lint" => "lint",
        "typecheck" => "typecheck",
        "build" => "build",
        "format" => "format",
        "dev_server" => "dev",
        _ => return None,
    })
}

fn edit_of(run: &KeptRun) -> Option<Edit> {
    let name = name_of(&run.kind)?.to_string();
    let command = match run.kind.as_str() {
        "dev_server" => NewCommand {
            run: None,
            destructive: false,
            serve: Some(run.command.clone()),
            ready: None,
            links: run
                .port
                .map(|port| vec![NewLink { url: format!("http://localhost:{port}"), name: Some("Dev server".to_string()) }])
                .unwrap_or_default(),
        },
        _ => NewCommand {
            run: Some(run.command.clone()),
            destructive: false,
            serve: None,
            ready: None,
            links: Vec::new(),
        },
    };
    Some(Edit::Command { name, edit: CommandEdit::Add(command) })
}

/// What a write did.
#[derive(Default)]
pub(crate) struct Written {
    /// Each run, with `Ok` where `armada.yml` now holds it and why not where it does not.
    pub(crate) standing: Vec<(KeptRun, Result<(), String>)>,
    /// The files changed, relative to the tree, to be committed.
    pub(crate) changed: Vec<String>,
}

/// Write `runs` into the `armada.yml` of each workspace under `tree`.
pub(crate) fn write(tree: &Path, runs: &[KeptRun]) -> Written {
    let mut written = Written::default();
    let mut by_workspace: BTreeMap<&str, Vec<&KeptRun>> = BTreeMap::new();
    for run in runs {
        by_workspace.entry(run.workspace.as_str()).or_default().push(run);
    }
    for (workspace, runs) in by_workspace {
        let relative = match workspace {
            "." => "armada.yml".to_string(),
            workspace => format!("{workspace}/armada.yml"),
        };
        let path = tree.join(&relative);
        let Ok(mut text) = std::fs::read_to_string(&path) else {
            let why = format!("{relative} does not exist");
            written.standing.extend(runs.into_iter().map(|run| (run.clone(), Err(why.clone()))));
            continue;
        };
        let mut newest: BTreeMap<&str, &KeptRun> = BTreeMap::new();
        for run in &runs {
            let held = newest.entry(run.kind.as_str()).or_insert(run);
            if run.last_ran > held.last_ran {
                *held = run;
            }
        }
        let mut changed = false;
        let mut standing = Vec::new();
        for run in runs {
            let outcome = match (newest.get(run.kind.as_str()), edit_of(run)) {
                (_, None) => Err(format!("`{}` is not a kind armada.yml holds", run.kind)),
                (Some(newest), _) if newest.command != run.command => {
                    Err(format!("a newer {} command is the one written", run.kind))
                }
                (_, Some(edit)) => match amend(&path, &text, &[edit]) {
                    Ok(amended) => {
                        changed |= amended.text() != text;
                        text = amended.text().to_string();
                        Ok(())
                    }
                    Err(NotAmended::Misnamed { key, declared: true }) => {
                        Err(format!("armada.yml already declares `{key}`"))
                    }
                    Err(other) => Err(other.to_string()),
                },
            };
            standing.push((run.clone(), outcome));
        }
        if changed {
            match std::fs::write(&path, &text) {
                Ok(()) => written.changed.push(relative),
                Err(why) => {
                    let why = format!("{relative} could not be written: {why}");
                    standing.iter_mut().for_each(|(_, outcome)| *outcome = Err(why.clone()));
                }
            }
        }
        written.standing.extend(standing);
    }
    written
}
