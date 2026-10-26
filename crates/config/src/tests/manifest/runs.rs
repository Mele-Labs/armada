//! `runs:`, and the `dir` a server starts in — `docs/concepts/manifest.md`, *Commands that keep
//! running*.

use super::parse;
use crate::error::Fault;
use crate::tests::{fault_at, refusals};

const APP: &str = "version: 1\nid: budget-app\nports:\n  api: {}\n  web: {}\ncommands:\n  api:\n    \
     serve: .venv/bin/python -m uvicorn main:app --port ${port.api}\n    dir: backend\n  web:\n    \
     serve: npx vite --port ${port.web} --strictPort\n    dir: frontend\n    links:\n      - url: \
     http://localhost:${port.web}\n  lint:\n    run: ruff check\n";

#[test]
fn a_run_lists_the_servers_it_starts_in_the_order_the_file_wrote_them() {
    let text = format!(
        "{APP}runs:\n  app:\n    label: Budget app\n    commands: [web, api]\n  backend:\n    \
         commands: [api]\n"
    );
    let manifest = parse(&text).expect("runs parse");
    assert_eq!(manifest.app_run_names(), ["app", "backend"]);
    let app = manifest.app_run("app").expect("app");
    assert_eq!(app.servers(), ["web", "api"], "the file's order, not sorted");
    assert_eq!(app.label(), Some("Budget app"));
    assert_eq!(manifest.app_run("backend").expect("backend").label(), None);
    assert!(manifest.app_run("nothing").is_none());
}

#[test]
fn a_file_with_no_runs_has_none() {
    let manifest = parse(APP).expect("parses");
    assert!(manifest.app_run_names().is_empty());
}

#[test]
fn a_run_naming_a_command_nothing_declares_is_refused_with_what_is_declared() {
    let refused = refusals(parse(&format!("{APP}runs:\n  app:\n    commands: [api, wbe]\n")));
    assert_eq!(
        fault_at(&refused, "runs.app.commands[1]"),
        &Fault::NotADeclaredServer {
            value: String::from("wbe"),
            declared: vec![String::from("api"), String::from("web")],
        }
    );
}

#[test]
fn a_run_naming_a_command_that_exits_is_refused_as_a_command() {
    let refused = refusals(parse(&format!("{APP}runs:\n  app:\n    commands: [lint]\n")));
    assert_eq!(
        fault_at(&refused, "runs.app.commands[0]"),
        &Fault::RunStartsACommand {
            value: String::from("lint")
        }
    );
}

#[test]
fn a_run_listing_a_server_twice_or_none_at_all_is_refused() {
    let refused = refusals(parse(&format!("{APP}runs:\n  app:\n    commands: [api, api]\n")));
    assert_eq!(
        fault_at(&refused, "runs.app.commands[1]"),
        &Fault::RunListsTwice { first_at: 0 }
    );
    let refused = refusals(parse(&format!("{APP}runs:\n  app:\n    label: Nothing\n")));
    assert_eq!(fault_at(&refused, "runs.app.commands"), &Fault::Missing);
    let refused = refusals(parse(&format!("{APP}runs:\n  app:\n    commands: []\n")));
    assert_eq!(fault_at(&refused, "runs.app.commands"), &Fault::Empty);
}

#[test]
fn a_run_with_a_key_nothing_reads_is_refused() {
    let refused = refusals(parse(&format!(
        "{APP}runs:\n  app:\n    commands: [api]\n    port: 80\n"
    )));
    assert!(matches!(
        fault_at(&refused, "runs.app.port"),
        Fault::Unknown { .. }
    ));
}

#[test]
fn a_server_says_the_folder_it_starts_in_and_a_missing_dir_is_the_root() {
    let manifest = parse(APP).expect("parses");
    assert_eq!(manifest.server("api").expect("api").dir(), Some("backend"));
    let plain = parse("version: 1\nid: x\ncommands:\n  web:\n    serve: node s.js\n").expect("parses");
    assert_eq!(plain.server("web").expect("web").dir(), None);
}

#[test]
fn a_dir_that_leaves_the_checkout_or_names_all_of_it_is_refused() {
    for dir in ["/etc", "../elsewhere", "a/../../b", "."] {
        let refused = refusals(parse(&format!(
            "version: 1\nid: x\ncommands:\n  web:\n    serve: node s.js\n    dir: {dir}\n"
        )));
        assert!(
            matches!(
                fault_at(&refused, "commands.web.dir"),
                Fault::NotAServeDir { .. }
            ),
            "{dir}"
        );
    }
}

#[test]
fn dir_on_a_command_that_exits_is_refused() {
    let refused = refusals(parse(
        "version: 1\nid: x\ncommands:\n  fmt:\n    run: cargo fmt\n    dir: crates\n",
    ));
    assert_eq!(
        fault_at(&refused, "commands.fmt.dir"),
        &Fault::OnlyAServerStartsIn
    );
}
