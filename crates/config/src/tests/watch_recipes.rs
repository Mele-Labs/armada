//! A watch recipe's own rules, over text, and which copy of one runs.

use std::path::{Path, PathBuf};

use crate::watches::{parse, RecipeSource, Recipes, WhyLeftOut, Written};

const QA: &str = "name: qa-deploy\ncommand: [scripts/watch/deploy-state, \"{sha}\", \"{environment}\"]\nevery: 60s\ngives_up_after: 2h\n";

fn refused(text: &str) -> String {
    parse(Path::new("w.yml"), text).expect_err("it should not parse").to_string()
}

#[test]
fn the_designs_recipe_loads_with_its_values_in_order() {
    let recipe = parse(Path::new("w.yml"), QA).expect("the design's own example");
    assert_eq!(recipe.name, "qa-deploy");
    assert_eq!(recipe.command, ["scripts/watch/deploy-state", "{sha}", "{environment}"]);
    assert_eq!(recipe.every_seconds, 60);
    assert_eq!(recipe.gives_up_after_seconds, Some(7200));
    assert_eq!(recipe.values(), ["sha", "environment"]);
    assert_eq!(recipe.environment, None);
}

#[test]
fn a_recipe_may_name_a_requests_environment() {
    let recipe = parse(Path::new("w.yml"), "name: ci\ncommand: [gh]\nevery: 1m\nenvironment: qa\n").unwrap();
    assert_eq!(recipe.environment.as_deref(), Some("qa"));
}

#[test]
fn gives_up_after_may_be_left_to_the_machine() {
    let recipe = parse(Path::new("w.yml"), "name: ci\ncommand: [gh, run, view]\nevery: 2m\n").unwrap();
    assert_eq!(recipe.gives_up_after_seconds, None);
    assert!(recipe.values().is_empty());
}

#[test]
fn a_command_is_a_list_and_never_a_line_for_a_shell() {
    let said = refused("name: ci\ncommand: gh run view | jq .status\nevery: 1m\n");
    assert!(said.contains("command"), "{said}");
    let said = refused("name: ci\ncommand: []\nevery: 1m\n");
    assert!(said.contains("command"), "{said}");
}

#[test]
fn a_span_is_a_count_and_a_unit_and_every_fault_is_reported() {
    let said = refused("name: ci\ncommand: [gh]\nevery: often\ngives_up_after: 3d\nenvironment: [qa]\nwhen: now\n");
    for key in ["every", "gives_up_after", "environment", "when"] {
        assert!(said.contains(key), "{key} is not named in: {said}");
    }
}

fn written(source: RecipeSource, path: &str, text: &str) -> Written {
    match source {
        RecipeSource::Machine => Written::on_machine(PathBuf::from(path), text.to_string()),
        _ => Written::in_repository(PathBuf::from(path), text.to_string()),
    }
}

#[test]
fn the_machines_copy_replaces_the_repositorys_whole() {
    let machine = "name: qa-deploy\ncommand: [/Users/user/bin/deploy-state, \"{sha}\"]\nevery: 30s\n";
    let recipes = Recipes::of([
        written(RecipeSource::Machine, "~/.armada/machine/watches/qa.yml", machine),
        written(RecipeSource::Repository, ".armada/watches/qa.yml", QA),
    ]);
    let one = recipes.named("qa-deploy").expect("one copy runs");
    assert_eq!(one.source, RecipeSource::Machine);
    assert_eq!(one.recipe.gives_up_after_seconds, None, "nothing is merged from the copy replaced");
    assert_eq!(one.replaced, [(RecipeSource::Repository, PathBuf::from(".armada/watches/qa.yml"))]);
    assert!(recipes.left_out().is_empty());
}

#[test]
fn two_files_in_one_place_naming_one_recipe_are_both_left_out_with_why() {
    let recipes = Recipes::of([
        written(RecipeSource::Repository, ".armada/watches/a.yml", QA),
        written(RecipeSource::Repository, ".armada/watches/b.yml", QA),
        written(RecipeSource::Repository, ".armada/watches/c.yml", "name: broken\n"),
    ]);
    assert!(recipes.recipes().is_empty());
    assert_eq!(recipes.left_out().len(), 2);
    assert!(recipes.left_out().iter().any(|one| matches!(one.why(), WhyLeftOut::Duplicated { .. })));
    let said: Vec<String> = recipes.left_out().iter().map(ToString::to_string).collect();
    assert!(said.iter().any(|one| one.contains("c.yml")), "{said:?}");
}
