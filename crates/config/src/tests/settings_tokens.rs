//! The token-use settings: what each ships, the range the compaction window is held to, and the two
//! per-repository maps taken whole, an empty one included.

use store::settings_file::{SettingValue, SettingsDocument};

use crate::settings::{self as s, checked, entry, Env, Group, Kind, Resolved, Shipped, Supplied};

fn resolved(pairs: &[(&str, SettingValue)]) -> Result<Resolved, String> {
    let supplied = s::prompt_keys().fold(Supplied::new(), |supplied, key| supplied.words(key, "a prompt"));
    let supplied = supplied
        .integer(s::CHECKS_AT_ONCE, 4)
        .words(s::HARNESS_AGENT, "the-cli")
        .words(s::HARNESS_BINARY_PATH, "the-cli")
        .list(s::MODELS_ROSTER, &["big", "middle", "small"])
        .words(s::MODELS_DEFAULT, "big")
        .words(s::MODELS_SESSION, "big")
        .words(s::MODELS_REVIEW, "big")
        .words(s::MODELS_JUDGE, "small")
        .words(s::MODELS_PROPOSER, "small")
        .words(s::MODELS_RETRO, "middle")
        .words(s::MODELS_SECOND_OPINION, "middle")
        .words(s::MODELS_DISPUTE, "small")
        .words(s::MODELS_TEST_MAP, "small")
        .words(s::MODELS_ACCOUNT, "small")
        .words(s::PLAN_DOWNSHIFT_TO, "middle")
        .harnesses(&["the-cli"]);
    let document: SettingsDocument = pairs.iter().map(|(key, value)| (key.to_string(), value.clone())).collect();
    let saved = checked(&document, &supplied, &Env::none()).map_err(|refused| refused.reason)?;
    Ok(Resolved::new(saved, supplied, Env::none()))
}

#[test]
fn the_token_use_settings_ship_as_the_walk_drew_them_in_the_fleet_group() {
    let nothing = resolved(&[]).expect("taken");
    assert_eq!(nothing.get(s::COMPACTION_WINDOW), 250_000);
    assert!(nothing.get(s::SUMMARY_SHORT_CACHE), "a short cache ships on");
    assert!(!nothing.get(s::SUMMARY_THINKING), "thinking ships off");
    assert_eq!(nothing.get(s::SKILLS_OFF), None, "no skill is off");
    assert_eq!(nothing.get(s::CONNECTORS_OFF), None, "no repository is told, so each follows its armada.yml");
    for key in [
        "tokens.compactionWindow",
        "tokens.summaryShortCache",
        "tokens.summaryThinking",
        "tokens.skillsOff",
        "tokens.connectorsOff",
    ] {
        let found = entry(key).expect(key);
        assert_eq!((found.section.title(), found.section.group()), ("Token use", Group::Fleet), "{key}");
        assert_eq!(found.shipped == Shipped::Nothing, matches!(found.kind, Kind::Json), "{key}");
    }
}

#[test]
fn the_compaction_window_is_held_to_the_range_the_cli_reads() {
    for ok in [100_000, 250_000, 1_000_000] {
        let taken = resolved(&[("tokens.compactionWindow", SettingValue::Integer(ok))]).expect("taken");
        assert_eq!(taken.get(s::COMPACTION_WINDOW), ok);
    }
    for refused in [99_999, 1_000_001, 0] {
        let why = resolved(&[("tokens.compactionWindow", SettingValue::Integer(refused))]).expect_err("refused");
        assert!(why.contains("tokens.compactionWindow"), "{why}");
    }
    assert!(resolved(&[("tokens.compactionWindow", SettingValue::Text("big".into()))]).is_err());
}

#[test]
fn the_summary_switches_take_a_boolean_and_nothing_else() {
    for key in ["tokens.summaryShortCache", "tokens.summaryThinking"] {
        assert!(resolved(&[(key, SettingValue::Bool(false))]).is_ok(), "{key}");
        assert!(resolved(&[(key, SettingValue::Text("yes".into()))]).is_err(), "{key}");
    }
    let off = resolved(&[("tokens.summaryShortCache", SettingValue::Bool(false))]).expect("taken");
    assert!(!off.get(s::SUMMARY_SHORT_CACHE));
}

#[test]
fn the_two_per_repository_maps_are_taken_whole_and_an_empty_one_is_taken() {
    let skills = SettingValue::from_json_text(r#"{"01REPO":["pptx","acme-skills:docx"]}"#).expect("JSON");
    let connectors = SettingValue::from_json_text(r#"{"01REPO":true,"01OTHER":false}"#).expect("JSON");
    let taken = resolved(&[("tokens.skillsOff", skills.clone()), ("tokens.connectorsOff", connectors.clone())]).expect("taken");
    assert_eq!(taken.get(s::SKILLS_OFF), Some(skills));
    assert_eq!(taken.get(s::CONNECTORS_OFF), Some(connectors));

    let empty = SettingValue::from_json_text("{}").expect("JSON");
    let taken = resolved(&[("tokens.skillsOff", empty.clone()), ("tokens.connectorsOff", empty.clone())]).expect("taken");
    assert_eq!(taken.get(s::SKILLS_OFF), Some(empty.clone()));
    assert_eq!(taken.get(s::CONNECTORS_OFF), Some(empty));
}
