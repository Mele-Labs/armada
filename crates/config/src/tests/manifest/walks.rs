//! `walks`, how a repository's app is built, listed, opened and pictured as a walk.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

const HEAD: &str = "version: 1\nid: a\n";

const WALKS: &str = "walks:\n  build: make app\n  binary: out/app\n  list: ${binary} --list\n  \
                     open: ${binary} play ${walk} ${job} ${url} ${fleet_port}\n  \
                     take: --take ${name}=${walk}\n  shots: ${binary} snap ${walk} ${shots}\n  \
                     against: ${binary} snap ${walk} ${shots} ${against} ${out}\n";

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

fn with(replace: &str, line: &str) -> String {
    let start = WALKS.find(&format!("  {replace}:")).expect("a key");
    let end = WALKS[start..].find('\n').map(|at| start + at + 1).expect("a line");
    format!("{HEAD}{}{line}{}", &WALKS[..start], &WALKS[end..])
}

#[test]
fn a_file_that_declares_no_walks_has_none() {
    assert!(parse(HEAD).expect("a bare manifest").walks().is_none());
}

#[test]
fn a_declaration_arrives_as_the_file_wrote_it() {
    let manifest = parse(&format!("{HEAD}{WALKS}")).expect("declared");
    let walks = manifest.walks().expect("walks");
    assert_eq!(walks.build(), "make app");
    assert_eq!(walks.binary(), "out/app");
    assert_eq!(walks.list(), "${binary} --list");
    assert_eq!(walks.open(), "${binary} play ${walk} ${job} ${url} ${fleet_port}");
    assert_eq!(walks.take(), Some("--take ${name}=${walk}"));
    assert_eq!(walks.shots(), "${binary} snap ${walk} ${shots}");
    assert_eq!(walks.against(), "${binary} snap ${walk} ${shots} ${against} ${out}");
}

#[test]
fn a_take_is_the_one_key_that_may_be_left_out() {
    let without = with("take", "");
    assert_eq!(parse(&without).expect("no takes").walks().expect("walks").take(), None);
    for key in ["build", "binary", "list", "open", "shots", "against"] {
        let refused = refusals(parse(&with(key, "")));
        assert_eq!(fault_at(&refused, &format!("walks.{key}")), &Fault::Missing, "{key}");
    }
}

#[test]
fn a_name_the_command_is_not_given_is_refused_with_the_names_it_is() {
    let refused = refusals(parse(&with("list", "  list: ${binary} ${walk}\n")));
    assert!(matches!(
        fault_at(&refused, "walks.list"),
        Fault::NotAPlaceholder { value, legal } if value == "${walk}" && legal == &["binary"]
    ));
    let build = refusals(parse(&with("build", "  build: make ${binary}\n")));
    assert!(matches!(fault_at(&build, "walks.build"), Fault::NotAPlaceholder { legal, .. } if legal.is_empty()));
}

#[test]
fn a_name_left_open_is_refused() {
    let refused = refusals(parse(&with("list", "  list: ${binary --list\n")));
    assert!(matches!(
        fault_at(&refused, "walks.list"),
        Fault::NotAPlaceholder { value, .. } if value == "${binary --list"
    ));
}

#[test]
fn a_command_that_never_names_what_it_needs_is_refused() {
    let refused = refusals(parse(&with("list", "  list: ./app --list\n")));
    assert_eq!(fault_at(&refused, "walks.list"), &Fault::LeavesOutPlaceholder { placeholder: "binary" });
    let open = refusals(parse(&with("open", "  open: ${binary} ${job} ${url} ${fleet_port}\n")));
    assert_eq!(fault_at(&open, "walks.open"), &Fault::LeavesOutPlaceholder { placeholder: "walk" });
}

#[test]
fn a_binary_that_leaves_the_tree_is_refused() {
    for value in ["/usr/bin/app", "../app", "out/*"] {
        let refused = refusals(parse(&with("binary", &format!("  binary: {value}\n"))));
        assert!(matches!(fault_at(&refused, "walks.binary"), Fault::NotAWalkPath { .. }), "{value}");
    }
}

#[test]
fn a_seed_is_optional_and_is_a_path_inside_the_tree() {
    let manifest = parse(&format!("{HEAD}{WALKS}  seed: out\n")).expect("seeded");
    assert_eq!(manifest.walks().expect("walks").seed(), Some("out"));
    assert_eq!(parse(&format!("{HEAD}{WALKS}")).expect("unseeded").walks().expect("walks").seed(), None);
    let refused = refusals(parse(&format!("{HEAD}{WALKS}  seed: ../out\n")));
    assert!(matches!(fault_at(&refused, "walks.seed"), Fault::NotAWalkPath { .. }));
}

#[test]
fn a_key_nothing_reads_is_refused() {
    let refused = refusals(parse(&format!("{HEAD}{WALKS}  serve: ${{binary}}\n")));
    assert!(matches!(fault_at(&refused, "walks.serve"), Fault::Unknown { .. }));
}

#[test]
fn the_walks_are_the_roots_to_say_and_a_workspace_that_declares_them_is_refused() {
    let root = parse(HEAD).expect("the root");
    let refused = refusals(Manifest::parse_workspace(
        &named("packages/a/armada.yml"),
        "packages/a",
        &format!("{HEAD}{WALKS}"),
        &root,
    ));
    assert_eq!(fault_at(&refused, "walks"), &Fault::RootOnly);
}
