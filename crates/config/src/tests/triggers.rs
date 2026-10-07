//! A Trigger file's own rules, over text.

use std::path::{Path, PathBuf};

use core_model::{TriggerRuns, TriggerWhen};

use crate::triggers::{parse, Catalogue, WhyLeftOut, Written};

fn refused(text: &str) -> String {
    parse(Path::new("t.yml"), text)
        .expect_err("it should not parse")
        .to_string()
}

#[test]
fn a_skill_is_modelled_and_failure_defaults_to_neither_flag() {
    let one = parse(
        Path::new("t.yml"),
        "name: r\nwhen: pr_opened\nskill: review\n",
    )
    .unwrap();
    assert_eq!(one.runs(), &TriggerRuns::Skill("review".to_string()));
    assert_eq!(one.when(), TriggerWhen::PrOpened);
    assert!(!one.on_failure().block && !one.on_failure().repair);
}

#[test]
fn it_runs_one_thing_and_pr_opened_names_no_step() {
    assert!(refused("name: r\nwhen: step_starts\ncommand: a\nskill: b\n").contains("not both"));
    assert!(refused("name: r\nwhen: step_starts\n").contains("neither"));
    assert!(refused("name: r\nwhen: pr_opened\nstep: x\ncommand: a\n").contains("`step`"));
}

#[test]
fn every_fault_is_named_and_an_unknown_key_refuses() {
    let why = refused("when: soon\ncommand: a\nflavour: x\non_failure: {block: 1}\n");
    for key in ["`name`", "`when`", "`flavour`", "`on_failure.block`"] {
        assert!(why.contains(key), "{key} in {why}");
    }
}

#[test]
fn two_files_of_one_identity_in_one_place_are_both_left_out() {
    let text = "name: t\nwhen: step_passes\ncommand: a\n";
    let at = |file: &str| PathBuf::from(file);
    let manifest = crate::Manifest::parse(Path::new("armada.yml"), "version: 1\nid: x\n").unwrap();
    let resolved = Catalogue::of([
        Written::in_repository(at("a.yml"), text.to_string()),
        Written::in_repository(at("b.yml"), text.to_string()),
        Written::on_machine(at("c.yml"), text.to_string()),
    ])
    .resolve(&manifest);
    assert_eq!(resolved.left_out().len(), 1);
    let WhyLeftOut::Duplicated { also } = resolved.left_out()[0].why() else {
        panic!("not a duplicate");
    };
    assert_eq!(also, &[at("b.yml")]);
    assert_eq!(resolved.triggers().len(), 1, "the machine's stands");
}
