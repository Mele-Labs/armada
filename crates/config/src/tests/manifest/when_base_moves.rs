//! `when_base_moves`, the Commands Fleet runs in the checkout once the base moves there.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

const COMMANDS: &str = "version: 1\nid: a\ncommands:\n  graph:\n    run: pnpm graph:index\n  \
                        docs:\n    run: cargo xtask docs\n  reset:\n    run: git reset --hard\n    \
                        destructive: true\n";

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn a_file_that_names_nothing_runs_nothing_when_the_base_moves() {
    assert!(parse(COMMANDS).expect("bare").when_base_moves().is_empty());
}

#[test]
fn the_commands_named_arrive_resolved_in_the_order_written() {
    let manifest = parse(&format!("{COMMANDS}when_base_moves: [docs, graph]\n")).expect("two");
    let runs: Vec<(&str, &str)> = manifest
        .when_base_moves()
        .iter()
        .map(|one| (one.name(), one.run()))
        .collect();
    assert_eq!(runs, [("docs", "cargo xtask docs"), ("graph", "pnpm graph:index")]);
}

#[test]
fn a_name_the_commands_do_not_declare_is_refused() {
    let refused = refusals(parse(&format!("{COMMANDS}when_base_moves: [index]\n")));
    assert!(matches!(
        fault_at(&refused, "when_base_moves[0]"),
        Fault::NotADeclaredCommand { value, .. } if value == "index"
    ));
}

#[test]
fn a_destructive_command_is_refused_when_the_base_moves() {
    let refused = refusals(parse(&format!("{COMMANDS}when_base_moves: [graph, reset]\n")));
    assert!(matches!(
        fault_at(&refused, "when_base_moves[1]"),
        Fault::RequiresSomethingDestructive { value } if value == "reset"
    ));
}

#[test]
fn an_empty_list_is_refused() {
    let refused = refusals(parse(&format!("{COMMANDS}when_base_moves: []\n")));
    assert_eq!(fault_at(&refused, "when_base_moves"), &Fault::Empty);
}

#[test]
fn a_word_in_place_of_a_list_is_refused() {
    let refused = refusals(parse(&format!("{COMMANDS}when_base_moves: graph\n")));
    assert!(matches!(
        fault_at(&refused, "when_base_moves"),
        Fault::WrongType { .. }
    ));
}

#[test]
fn when_the_base_moves_is_the_roots_to_say_and_a_workspace_that_says_so_is_refused() {
    let root = parse(COMMANDS).expect("the root");
    let refused = refusals(Manifest::parse_workspace(
        &named("packages/a/armada.yml"),
        "packages/a",
        "version: 1\nid: b\nwhen_base_moves: [graph]\n",
        &root,
    ));
    assert_eq!(fault_at(&refused, "when_base_moves"), &Fault::RootOnly);
}
