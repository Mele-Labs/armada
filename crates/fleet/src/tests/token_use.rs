//! What a Job is started with to cost fewer tokens, read from the settings: connectors over what
//! `armada.yml` says, the skills a repository's Jobs start without, and how a turn summary is asked for.

use std::collections::BTreeMap;

use adapter_traits::{Ask, Environment, Model};
use store::settings_file::SettingValue;

use super::tmp::TempDir;
use crate::session_host::account::summary_asked;
use crate::session_host::token_use::{connectors_on, skills_off};

fn map(pairs: Vec<(&str, SettingValue)>) -> Option<SettingValue> {
    Some(SettingValue::Map(pairs.into_iter().map(|(key, value)| (key.to_string(), value)).collect::<BTreeMap<_, _>>()))
}

fn names(names: &[&str]) -> SettingValue {
    SettingValue::List(names.iter().map(|name| SettingValue::Text(name.to_string())).collect())
}

#[test]
fn the_persons_connectors_switch_wins_over_armada_yml_in_both_directions() {
    let off = map(vec![("repo", SettingValue::Bool(true))]);
    let on = map(vec![("repo", SettingValue::Bool(false))]);
    assert!(!connectors_on(off.clone(), "repo", true), "off although armada.yml allows them");
    assert!(connectors_on(on.clone(), "repo", false), "on although armada.yml says false");
    assert!(connectors_on(off, "another", true), "no entry follows armada.yml: on");
    assert!(!connectors_on(on, "another", false), "no entry follows armada.yml: off");
}

#[test]
fn a_connectors_value_that_is_not_the_shape_follows_armada_yml() {
    assert!(connectors_on(None, "repo", true));
    assert!(!connectors_on(None, "repo", false));
    assert!(connectors_on(Some(names(&["repo"])), "repo", true), "a list is what an older build saved");
    assert!(!connectors_on(map(vec![("repo", SettingValue::Text("off".into()))]), "repo", false));
}

#[test]
fn skills_off_for_a_repository_leave_out_its_own_skills_and_the_rest_are_kept() {
    let root = TempDir::new();
    std::fs::create_dir_all(root.path().join(".claude/skills/release-notes")).expect("a skill folder");
    let off = map(vec![
        ("repo", names(&["pptx", "release-notes", "anthropic-skills:docx", "armada:root-cause"])),
        ("another", names(&["other"])),
    ]);
    // `armada:` ones are the adapter's to refuse; a repository's own, which only Fleet can find, are left out here.
    assert_eq!(
        skills_off(off.clone(), "repo", Some(root.path())),
        ["pptx", "anthropic-skills:docx", "armada:root-cause"]
    );
    assert_eq!(skills_off(off.clone(), "another", Some(root.path())), ["other"]);
    assert!(skills_off(off.clone(), "none", Some(root.path())).is_empty());
    assert!(skills_off(None, "repo", Some(root.path())).is_empty());
    assert!(skills_off(map(vec![("repo", SettingValue::Bool(true))]), "repo", None).is_empty());
    assert_eq!(skills_off(off, "repo", None).len(), 4, "no checkout, nothing to find");
}

fn ask() -> Ask {
    Ask::put(Model::named("haiku").expect("a model"), "what did the turn do?", Environment::nothing()).expect("an ask")
}

#[test]
fn the_turn_summary_ask_honours_each_switch_alone() {
    let plain = summary_asked(ask(), false, false);
    assert!(!plain.is_kept_briefly() && !plain.is_unthinking());
    let briefly = summary_asked(ask(), true, false);
    assert!(briefly.is_kept_briefly() && !briefly.is_unthinking());
    let unthinking = summary_asked(ask(), false, true);
    assert!(!unthinking.is_kept_briefly() && unthinking.is_unthinking());
    let both = summary_asked(ask(), true, true);
    assert!(both.is_kept_briefly() && both.is_unthinking());
}
