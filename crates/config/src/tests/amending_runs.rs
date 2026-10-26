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

#[test]
fn a_copy_that_names_only_what_it_changes_takes_a_run_by_splice_with_its_comments_and_loads_laid_over_a_file() {
    let kit = "# Mine.\nchecks:\n  test:\n    run: make quick\n";
    let spliced = crate::amending::splice(kit, &app()).expect("they apply");
    assert!(spliced.starts_with("# Mine.\nchecks:\n  test:\n    run: make quick\n"), "{spliced}");
    assert_eq!(crate::kit_copy_fault(&spliced), None);
    let repository = "version: 1\nid: budget-app\nchecks:\n  test:\n    run: pytest -q\n";
    let manifest = crate::Manifest::with_kit(at(), repository, Path::new("/kit/budget-app.yml"), &spliced).expect("it merges");
    assert!(manifest.from_kit().is_some_and(|from| from.left_out.is_none()));
    assert_eq!(manifest.app_run("app").map(|run| run.servers().len()), Some(2));
    assert_eq!(manifest.check("test").map(|check| check.run().to_string()).as_deref(), Some("make quick"));
}
