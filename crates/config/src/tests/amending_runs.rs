//! A run, and a server's `dir`, written into an `armada.yml` by the form writer.

use std::path::Path;

use crate::amending::{
    amend, CommandEdit, Edit, NewCommand, NewPort, NewRun, NotAmended, PortEdit, RunEdit,
};

const OWN: &str = include_str!("../../../../armada.yml");

fn at() -> &'static Path {
    Path::new("armada.yml")
}

fn app() -> Vec<Edit> {
    let server = |name: &str, serve: &str, dir: &str| {
        vec![
            Edit::Port {
                name: name.to_string(),
                edit: PortEdit::Add(NewPort {
                    container: None,
                    env: None,
                }),
            },
            Edit::Command {
                name: name.to_string(),
                edit: CommandEdit::Add(NewCommand {
                    run: None,
                    destructive: false,
                    serve: Some(serve.to_string()),
                    ready: None,
                    links: Vec::new(),
                }),
            },
            Edit::Command {
                name: name.to_string(),
                edit: CommandEdit::Dir(Some(dir.to_string())),
            },
        ]
    };
    let mut edits = server("api", "uvicorn main:app --port ${port.api}", "backend");
    edits.extend(server("web", "npx vite --port ${port.web}", "frontend"));
    edits.push(Edit::Run {
        name: "app".to_string(),
        edit: RunEdit::Add(NewRun {
            label: Some("Budget app".to_string()),
            commands: vec!["api".to_string(), "web".to_string()],
        }),
    });
    edits
}

#[test]
fn a_run_and_its_servers_are_written_into_a_file_that_loads_as_them() {
    let text = "version: 1\nid: budget-app\n";
    let done = amend(at(), text, &app()).expect("they apply");
    let manifest = done.manifest();
    let run = manifest.app_run("app").expect("the run");
    assert_eq!(run.servers(), ["api", "web"]);
    assert_eq!(run.label(), Some("Budget app"));
    assert_eq!(
        manifest.server("web").expect("web").dir(),
        Some("frontend")
    );
}

#[test]
fn a_run_added_to_this_repositorys_own_file_leaves_every_comment_where_it_was() {
    let done = amend(at(), OWN, &app()).expect("they apply to the real file");
    let comments = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| line.trim_start().starts_with('#'))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(comments(done.text()), comments(OWN), "no comment lost or moved");
    assert!(done.manifest().app_run("app").is_some());
}

#[test]
fn a_run_named_twice_or_listing_a_command_that_is_not_there_is_refused() {
    let text = "version: 1\nid: budget-app\n";
    let mut twice = app();
    twice.push(twice.last().cloned().expect("a run"));
    assert!(matches!(
        amend(at(), text, &twice),
        Err(NotAmended::Misnamed { declared: true, .. })
    ));
    let unknown = [Edit::Run {
        name: "app".to_string(),
        edit: RunEdit::Add(NewRun {
            label: None,
            commands: vec!["nothing".to_string()],
        }),
    }];
    assert!(matches!(
        amend(at(), text, &unknown),
        Err(NotAmended::Refused(_))
    ));
}

#[test]
fn a_run_goes_out_and_takes_its_section_when_it_was_the_last() {
    let done = amend(at(), "version: 1\nid: budget-app\n", &app()).expect("they apply");
    let removed = amend(
        at(),
        done.text(),
        &[Edit::Run {
            name: "app".to_string(),
            edit: RunEdit::Remove,
        }],
    )
    .expect("removed");
    assert!(removed.manifest().app_run_names().is_empty());
    assert!(!removed.text().contains("runs:"));
}
