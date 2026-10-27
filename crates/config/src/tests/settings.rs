//! The settings table: every entry sound, every typed handle naming an entry
//! of its own kind, a document refused whole on its first fault, and a value
//! resolved variable first, file second, default last.

use std::collections::BTreeMap;
use std::time::Duration;

use store::settings_file::{SettingValue, SettingsDocument};

use crate::settings::{
    self as s, checked, entries, entry, Env, Key, Kind, Resolved, Shipped, Supplied,
};

fn supplied() -> Supplied {
    let prompts = s::prompt_keys().fold(Supplied::new(), |supplied, key| {
        supplied.words(key, "a prompt")
    });
    prompts
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
        .harnesses(&["the-cli"])
}

fn document(pairs: &[(&str, SettingValue)]) -> SettingsDocument {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.clone()))
        .collect()
}

fn refusal(pairs: &[(&str, SettingValue)]) -> (Option<String>, String) {
    let refused = checked(&document(pairs), &supplied(), &Env::none()).expect_err("refused");
    (refused.key, refused.reason)
}

#[test]
fn every_entry_is_named_once_in_its_section_and_ships_a_value_of_its_kind() {
    let mut seen = BTreeMap::new();
    for entry in entries() {
        assert!(
            seen.insert(entry.key, ()).is_none(),
            "`{}` twice",
            entry.key
        );
        let (prefix, rest) = entry.key.split_once('.').expect("dotted");
        assert!(
            rest.starts_with(|c: char| c.is_ascii_lowercase()),
            "{}",
            entry.key
        );
        assert!(!rest.contains(['.', '_', '-']), "camelCase: {}", entry.key);
        // A prompt's prefix names its group, Prompts; its section is the
        // family's sub-heading under it.
        let heading = match entry.section {
            s::Section::Prompt(_) => entry.section.group().title(),
            _ => entry.section.title(),
        };
        assert!(
            heading
                .to_lowercase()
                .starts_with(&prefix[..prefix.len().min(5)]),
            "`{}` drawn under {}",
            entry.key,
            entry.section.title()
        );
        // A setting drawn with no sentence names itself in its control's tooltip instead.
        assert!(
            (!entry.description.is_empty() || entry.tip.is_some_and(|tip| !tip.is_empty() && tip != entry.title))
                && !entry.title.is_empty(),
            "{}",
            entry.key
        );
        let fits = match (entry.kind, entry.shipped) {
            (_, Shipped::Supplied) => true,
            (Kind::Integer { min, max, .. }, Shipped::Integer(n)) => (min..=max).contains(&n),
            (Kind::Seconds { min, max }, Shipped::Seconds(n)) => (min..=max).contains(&n),
            (Kind::Boolean, Shipped::Boolean(_)) => true,
            (Kind::Choice(_), Shipped::Text(_))
            | (Kind::Text | Kind::Prompt(_), Shipped::Text(_)) => true,
            (Kind::TextList, Shipped::TextList(items)) => !items.is_empty(),
            (Kind::Json, Shipped::Nothing) => true,
            _ => false,
        };
        assert!(
            fits,
            "`{}` ships {:?} as {:?}",
            entry.key, entry.shipped, entry.kind
        );
    }
    assert!(
        supplied().missing().is_empty(),
        "{:?}",
        supplied().missing()
    );
}

#[test]
fn every_row_an_entry_realises_is_in_the_design_registry() {
    let registry = include_str!("../../settings.toml");
    for entry in entries() {
        if let Some(row) = entry.row {
            assert!(
                registry.contains(&format!("[settings.{row}]")),
                "`{}` names `{row}`",
                entry.key
            );
        }
    }
}

/// A handle that named no entry, or one of another kind, would read a default
/// of nothing. Each is listed here once, beside the kind it must be.
#[test]
fn every_handle_names_an_entry_of_its_kind() {
    let kinds = |key: &str| entry(key).map(|found| found.kind).expect(key);
    for key in [
        s::DRONES_AT_ONCE,
        s::CHECKS_AT_ONCE,
        s::MEMORY_SPARE_PERCENT,
        s::DISK_FLOOR_GIB,
        s::STANDING_RULES_KIB,
        s::SLOT_BUILD_TRIM_AFTER_DAYS,
        s::SLOT_BUILD_CEILING_GIB,
        s::PORT_RANGE_BASE,
        s::PORT_BLOCK_GRANULE,
        s::TURN_INTERVAL_MS,
        s::BRIDGE_RECONNECT_MS,
        s::RUN_LOG_DAYS,
        s::PLAN_DOWNSHIFT_AT,
        s::CONTEXT_WATCH_KTOKENS,
        s::SUBAGENT_WATCH_PERCENT,
        s::CACHE_WATCH_PERCENT,
        s::OPUS_READING_WATCH_TURNS,
        s::DISPUTE_READ_MINUTES,
        s::WATCHES_AT_ONCE,
        s::WATCHES_PER_JOB,
        s::WATCH_GIVE_UP_MINUTES,
    ] {
        assert!(
            matches!(kinds(key.name()), Kind::Integer { .. }),
            "{}",
            key.name()
        );
    }
    for key in [
        s::CHECK_SECONDS,
        s::JUDGE_SECONDS,
        s::COMMAND_SECONDS,
        s::PROPOSER_SECONDS,
        s::HELM_ASK_HOLD_SECONDS,
        s::SESSION_QUIET_SECONDS,
        s::HELM_REPLY_SECONDS,
        s::MERGE_NOTICE_SECONDS,
        s::RESOURCE_POLL_SECONDS,
        s::BUILD_SWEEP_SECONDS,
        s::WATCH_RUN_SECONDS,
        s::WATCH_EVERY_SECONDS,
        s::NOTICE_BATCH_SECONDS,
    ] {
        assert!(
            matches!(kinds(key.name()), Kind::Seconds { .. }),
            "{}",
            key.name()
        );
    }
    for key in [
        s::HELM_CAN_ACT,
        s::DRAFT_PULL_REQUESTS,
        s::OPEN_GUIDES_FIRST_TIME,
        s::WORKFLOWS,
        s::DISPUTE_REVIEWS,
        s::NOTIFICATIONS,
        s::UI_SOUNDS,
        s::DEEP_LINKS,
        s::AUTOPILOT_PANEL,
        s::WHERE_THINGS_ARE_OPEN,
        s::CONTEXT_WATCH,
        s::SUBAGENT_WATCH,
        s::CACHE_WATCH,
        s::OPUS_READING_WATCH,
    ] {
        assert_eq!(kinds(key.name()), Kind::Boolean, "{}", key.name());
    }
    for key in [
        s::HARNESS_AGENT,
        s::HARNESS_BINARY_PATH,
        s::MODELS_DEFAULT,
        s::MODELS_SESSION,
        s::MODELS_REVIEW,
        s::MODELS_JUDGE,
        s::MODELS_PROPOSER,
        s::MODELS_RETRO,
        s::MODELS_SECOND_OPINION,
        s::MODELS_DISPUTE,
        s::MODELS_TEST_MAP,
        s::MODELS_ACCOUNT,
        s::PLAN_DOWNSHIFT_TO,
        s::PLAN_CONTEXT_HANDOFF,
        s::EFFORT_DEFAULT,
        s::EFFORT_JUDGE,
        s::EFFORT_PROPOSER,
        s::EFFORT_HELM,
        s::EDITOR_COMMAND,
        s::TERMINAL_APP,
        s::TERMINAL_COMMAND,
        s::OPEN_JOB_TERMINAL_IN,
        s::OPEN_PULL_REQUESTS_IN,
        s::MACHINE_NAME,
        s::ACCOUNT_CLIENT_ID,
        s::STORE_URL,
        s::THEME,
    ] {
        assert!(
            matches!(kinds(key.name()), Kind::Text | Kind::Choice(_)),
            "{}",
            key.name()
        );
    }
    for key in [s::HARNESS_DRONE_PATH, s::MODELS_ROSTER] {
        assert_eq!(kinds(key.name()), Kind::TextList, "{}", key.name());
    }
    for key in [
        s::LAYOUT,
        s::KEY_BINDINGS,
        s::TIME_FORMAT,
        s::ROW_VIEW,
        s::REACH_FOLDERS,
    ] {
        assert_eq!(kinds(key.name()), Kind::Json, "{}", key.name());
    }
    for key in s::prompt_keys() {
        assert!(
            matches!(kinds(key.name()), Kind::Prompt(_)),
            "{}",
            key.name()
        );
    }
    let handles = 21 + 13 + 14 + 27 + 2 + 5 + s::prompt_keys().count();
    assert_eq!(
        entries().count(),
        handles,
        "an entry with no handle is a value nothing reads"
    );
}

#[test]
fn a_document_of_known_keys_in_range_is_taken_whole() {
    let pairs = [
        ("limits.dronesAtOnce", SettingValue::Integer(3)),
        ("timeouts.checkSeconds", SettingValue::Integer(600)),
        ("features.helmCanAct", SettingValue::Bool(false)),
        ("models.judge", SettingValue::Text("middle".into())),
        ("terminal.app", SettingValue::Text("Ghostty".into())),
        (
            "bridge.layout",
            SettingValue::from_json_text(r#"{"version":1}"#).expect("JSON"),
        ),
    ];
    let saved = checked(&document(&pairs), &supplied(), &Env::none()).expect("taken");
    assert_eq!(saved.document(), &document(&pairs));
}

#[test]
fn an_unknown_key_refuses_the_file_and_names_the_key() {
    let (key, reason) = refusal(&[
        ("limits.dronesAtOnce", SettingValue::Integer(3)),
        ("limits.dronesAtOnse", SettingValue::Integer(3)),
    ]);
    assert_eq!(key.as_deref(), Some("limits.dronesAtOnse"));
    assert_eq!(
        reason,
        "settings.json names `limits.dronesAtOnse`, which is not a setting Armada has"
    );
}

#[test]
fn a_value_out_of_range_or_of_another_kind_is_refused_by_its_key() {
    assert_eq!(
        refusal(&[("limits.dronesAtOnce", SettingValue::Integer(12))]),
        (
            Some("limits.dronesAtOnce".to_string()),
            "`limits.dronesAtOnce` must be a whole number of Drones from 1 to 8; settings.json has `12`"
                .to_string()
        )
    );
    assert_eq!(
        refusal(&[("limits.dronesAtOnce", SettingValue::Float(2.5))])
            .0
            .as_deref(),
        Some("limits.dronesAtOnce")
    );
    assert!(
        refusal(&[("features.helmCanAct", SettingValue::Text("yes".into()))])
            .1
            .contains("true or false")
    );
    assert!(
        refusal(&[("models.judge", SettingValue::Text("enormous".into()))])
            .1
            .contains("must be one of big, middle, small")
    );
    assert!(
        refusal(&[("harness.dronePath", SettingValue::List(vec![]))])
            .1
            .contains("at least one item")
    );
}

/// A Job's terminal opens docked under its thread unless the side panel or a window of its own
/// is chosen, and Bridge reads a change on the next open.
#[test]
fn a_jobs_terminal_opens_docked_unless_the_side_panel_or_a_window_is_chosen() {
    let found = entry(s::OPEN_JOB_TERMINAL_IN.name()).expect("an entry");
    assert_eq!(found.section, s::Section::Terminal);
    assert_eq!(
        found.kind,
        Kind::Choice(s::Options::Fixed(&["docked", "side panel", "window"]))
    );
    assert_eq!(found.shipped, Shipped::Text("docked"));
    assert_eq!(found.applies, s::Applies::Live);
    for place in ["side panel", "window"] {
        let pairs = [("terminal.openJobTerminalIn", SettingValue::Text(place.into()))];
        assert!(checked(&document(&pairs), &supplied(), &Env::none()).is_ok(), "{place}");
    }
    let (refused, reason) = refusal(&[(
        "terminal.openJobTerminalIn",
        SettingValue::Text("tab".into()),
    )]);
    assert_eq!(refused.as_deref(), Some("terminal.openJobTerminalIn"));
    assert!(reason.contains("must be one of docked, side panel, window"), "{reason}");
}

/// The effort settings take every level the agent CLI does, and nothing else.
#[test]
fn an_effort_setting_takes_the_cli_levels_up_to_max_and_refuses_the_rest() {
    for key in ["effort.default", "effort.judge", "effort.proposer", "effort.helm"] {
        for level in ["harness default", "low", "medium", "high", "xhigh", "max"] {
            let pairs = [(key, SettingValue::Text(level.into()))];
            assert!(
                checked(&document(&pairs), &supplied(), &Env::none()).is_ok(),
                "{key} refused {level}"
            );
        }
        let (refused, reason) = refusal(&[(key, SettingValue::Text("ultra".into()))]);
        assert_eq!(refused.as_deref(), Some(key));
        assert!(reason.contains("must be one of"), "{reason}");
    }
}

/// A file that names a model and a roster together is checked against its own
/// roster, not the one it replaces.
#[test]
fn a_model_is_checked_against_the_roster_the_same_file_names() {
    let pairs = [
        (
            "models.roster",
            SettingValue::List(vec![SettingValue::Text("enormous".into())]),
        ),
        ("models.judge", SettingValue::Text("enormous".into())),
    ];
    assert!(checked(&document(&pairs), &supplied(), &Env::none()).is_ok());
}

/// The account writer's model is its own dial, so raising the Judge does not raise it.
#[test]
fn the_account_writers_model_is_a_setting_and_a_model_off_the_roster_is_refused() {
    let saved = checked(&document(&[("models.account", SettingValue::Text("middle".into()))]), &supplied(), &Env::none())
        .expect("taken");
    let resolved = Resolved::new(saved, supplied(), Env::none());
    assert_eq!(resolved.get(s::MODELS_ACCOUNT), "middle");

    let (key, reason) = refusal(&[("models.account", SettingValue::Text("enormous".into()))]);
    assert_eq!(key.as_deref(), Some("models.account"));
    assert!(reason.contains("must be one of big, middle, small"), "{reason}");
}

#[test]
fn a_variable_wins_over_the_file_and_the_file_over_what_ships() {
    let saved = checked(
        &document(&[
            ("models.judge", SettingValue::Text("middle".into())),
            ("limits.dronesAtOnce", SettingValue::Integer(5)),
        ]),
        &supplied(),
        &Env::none(),
    )
    .expect("taken");
    let env = Env::read(|var| (var == "ARMADA_JUDGE_MODEL").then(|| "elsewhere".to_string()));
    let resolved = Resolved::new(saved, supplied(), env);

    assert_eq!(resolved.get(s::MODELS_JUDGE), "elsewhere");
    assert_eq!(
        resolved.chosen(s::MODELS_JUDGE),
        None,
        "the file gives way to the variable"
    );
    assert_eq!(
        resolved.overridden_by_env(entry("models.judge").expect("there")),
        Some("ARMADA_JUDGE_MODEL")
    );
    assert_eq!(resolved.get(s::DRONES_AT_ONCE), 5);
    assert_eq!(resolved.chosen(s::DRONES_AT_ONCE), Some(5));
    assert_eq!(
        resolved.get(s::CHECK_SECONDS),
        Duration::from_secs(900),
        "nothing saved is what ships"
    );
    assert_eq!(resolved.chosen(s::CHECK_SECONDS), None);
    assert_eq!(resolved.get(s::CHECKS_AT_ONCE), 4, "a supplied default");
    assert_eq!(resolved.get(s::LAYOUT), None);
}

#[test]
fn a_blank_variable_is_unset_and_the_default_model_variable_leads_the_roster() {
    let env = Env::read(|var| match var {
        "ARMADA_MODEL" => Some("newest".to_string()),
        "ARMADA_AGENT_BINARY" => Some("  ".to_string()),
        _ => None,
    });
    let resolved = Resolved::new(crate::settings::Saved::nothing(), supplied(), env);
    assert_eq!(resolved.models(), vec!["newest", "big", "middle", "small"]);
    assert_eq!(resolved.get(s::HARNESS_BINARY_PATH), "the-cli");
}

/// The index on Bridge's left lists groups in one order, and the table walks
/// them in that order so no group appears twice.
#[test]
fn entries_come_group_by_group_in_the_order_bridge_lists_them() {
    use crate::settings::Group;
    let order = [
        Group::Agents,
        Group::Prompts,
        Group::Fleet,
        Group::Features,
        Group::Tools,
        Group::Appearance,
    ];
    let ranks: Vec<usize> = entries()
        .map(|entry| {
            order
                .iter()
                .position(|group| *group == entry.section.group())
                .expect("ordered")
        })
        .collect();
    assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]), "{ranks:?}");
    assert_eq!(
        entry("bridge.layout").expect("there").section.group(),
        Group::Appearance
    );
}

/// A prompt a person saved keeps every placeholder Fleet fills, and is never
/// blank: a scout told no `{root}` does not know where it is.
#[test]
fn a_prompt_that_drops_a_placeholder_or_is_blank_is_refused() {
    let kept = SettingValue::Text("Read {root} and answer.".into());
    assert!(checked(
        &document(&[("prompts.scoutAsk", kept)]),
        &supplied(),
        &Env::none()
    )
    .is_ok());

    let (key, reason) = refusal(&[(
        "prompts.scoutAsk",
        SettingValue::Text("Read the code.".into()),
    )]);
    assert_eq!(key.as_deref(), Some("prompts.scoutAsk"));
    assert!(reason.contains("must keep {root}"), "{reason}");

    let (_, reason) = refusal(&[(
        "prompts.scoutRescue",
        SettingValue::Text("At {root} on {branch}.".into()),
    )]);
    assert!(
        reason.contains("{commit}, {base}"),
        "every one dropped is named: {reason}"
    );

    let (_, reason) = refusal(&[("prompts.retroWhereFixesLand", SettingValue::Text("  \n ".into()))]);
    assert!(reason.contains("must not be blank"), "{reason}");

    let (_, reason) = refusal(&[("prompts.retroWhereFixesLand", SettingValue::Integer(1))]);
    assert!(reason.contains("must be text"), "{reason}");
}

/// Every prompt sits in the Prompts group under its family's sub-heading, the
/// families run in the order a Drone meets them, and none is empty: Bridge
/// draws the sections in the order the list first names them.
#[test]
fn prompts_run_in_their_sections_in_the_order_a_drone_meets_them() {
    use crate::settings::{Group, Kind, PromptSection, Section};
    for (place, section) in PromptSection::ORDER.iter().enumerate() {
        assert_eq!(
            section.position(),
            place,
            "{} is out of place",
            section.title()
        );
    }
    let mut last = 0;
    let mut seen = std::collections::BTreeSet::new();
    let mut left_the_prompts = false;
    let mut in_the_prompts = false;
    for entry in s::entries() {
        let Kind::Prompt(_) = entry.kind else {
            left_the_prompts |= in_the_prompts;
            continue;
        };
        assert!(
            !left_the_prompts,
            "{} is apart from the other prompts",
            entry.key
        );
        in_the_prompts = true;
        let Section::Prompt(section) = entry.section else {
            panic!("{} is a prompt outside a prompt section", entry.key);
        };
        assert_eq!(entry.section.group(), Group::Prompts, "{}", entry.key);
        assert!(section.position() >= last, "{} breaks the order", entry.key);
        last = section.position();
        seen.insert(section.position());
    }
    for section in PromptSection::ORDER {
        assert!(
            seen.contains(&section.position()),
            "{} holds no prompt",
            section.title()
        );
    }
}

/// A prompt's title is a name a person reads, not its key: it starts with a
/// capital, spells no key, and is the only one of its name in its section.
#[test]
fn every_prompt_title_is_a_readable_name_and_not_its_key() {
    use crate::settings::Kind;
    let mut named = std::collections::BTreeSet::new();
    for entry in s::entries().filter(|entry| matches!(entry.kind, Kind::Prompt(_))) {
        let bare = entry.key.trim_start_matches("prompts.");
        let title = entry.title;
        assert!(
            title.starts_with(|first: char| first.is_uppercase()),
            "{}: {title:?}",
            entry.key
        );
        assert!(
            !title.contains("prompts.") && title != bare,
            "{}: {title:?}",
            entry.key
        );
        assert!(
            !title
                .chars()
                .zip(title.chars().skip(1))
                .any(|(one, two)| one.is_lowercase() && two.is_uppercase()),
            "{}: {title:?} reads as an identifier",
            entry.key
        );
        assert!(
            named.insert((entry.section.title(), title)),
            "{title:?} is twice in {}",
            entry.section.title()
        );
    }
}
