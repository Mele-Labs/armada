//! settings.json's sixth claim, over a real Fleet: a prompt a person saves is
//! what the next Drone's brief opens with, and removing it puts the shipped
//! prompt back. Beside `settings_json` for that module's reason.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::Settings;
use config::settings::{self as keys, checked, Env, Key};
use ipc::{SaveSettings, SettingValue};
use store::settings_file;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::briefing::BASELINE;
use crate::daemon::{Fittings, Fleet};
use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::settings_json::supplied;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const OVERRIDE: &str = "You work in a worktree of your own. Report with the evidence tool, once.";

fn fitted(home: &TempDir) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]),
    );
    fittings.starting().workflows = one(testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]));
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    fittings.concurrency = Concurrency::of(4);
    fittings
}

fn changing(key: &str, value: Option<SettingValue>) -> SaveSettings {
    SaveSettings {
        changes: BTreeMap::from([(key.to_string(), value)]),
    }
}

/// The brief the Drone put on `title` was spawned with.
async fn briefed(fleet: &Fixture, home: &TempDir, title: &str) -> String {
    let job = fleet.propose(a_proposal(title)).await.expect("a proposal");
    worktree_directory(home, &job);
    dispatched(fleet, job.id())
        .await
        .expect("a person approves it");
    let configured = fleet.harness().configured();
    configured
        .last()
        .map(|one| one.prompt().as_str().to_string())
        .expect("a Drone was spawned")
}

/// Claim 6: an override reaches the next Drone's brief, and `null` restores
/// the shipped prompt for the Drone after it.
#[tokio::test]
async fn a_saved_prompt_reaches_the_next_drone_and_removing_it_restores_the_shipped_one() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    let key = keys::PROMPT_DRONE_BASELINE.name();

    let first = briefed(&fleet, &home, "before any override").await;
    assert!(first.starts_with(BASELINE), "what ships: {first}");

    fleet
        .save_settings(changing(key, Some(SettingValue::Text(OVERRIDE.into()))))
        .await
        .expect("saved");
    let written = settings_file::read(&home.path().join("settings.json"))
        .expect("reads")
        .expect("written");
    assert_eq!(
        written.get(key),
        Some(&settings_file::SettingValue::Text(OVERRIDE.into()))
    );
    let second = briefed(&fleet, &home, "with the override").await;
    assert!(
        second.starts_with(OVERRIDE),
        "the override opens the brief: {second}"
    );
    assert!(!second.contains(BASELINE), "and replaces the shipped text");
    assert!(
        second.contains("Implement"),
        "the blocks Fleet writes still follow it"
    );

    fleet
        .save_settings(changing(key, None))
        .await
        .expect("saved");
    let third = briefed(&fleet, &home, "after the reset").await;
    assert!(
        third.starts_with(BASELINE),
        "the shipped prompt is back: {third}"
    );
}

/// The served default is the shipped prompt, so Bridge's "Reset to shipped"
/// shows the words a Drone gets.
#[tokio::test]
async fn the_default_served_for_a_prompt_is_what_fleet_ships() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    let served = fleet.get_settings().await.expect("reads");
    let baseline = served
        .settings
        .iter()
        .find(|one| one.key == "prompts.droneBaseline")
        .expect("listed");
    assert_eq!(baseline.default, SettingValue::Text(BASELINE.into()));
    assert_eq!(baseline.group, "Prompts");
    assert_eq!(
        baseline.section, "Drone brief",
        "the wire carries the sub-heading"
    );
}

/// Every prompt Fleet ships keeps its own placeholders, so what ships is a
/// value the table would take back from a person.
#[test]
fn every_shipped_prompt_is_one_the_table_accepts() {
    let shipped = supplied();
    for key in keys::prompt_keys() {
        let text = crate::prompts::Prompts::shipped().get(key).to_string();
        let document = BTreeMap::from([(
            key.name().to_string(),
            settings_file::SettingValue::Text(text),
        )]);
        if let Err(refused) = checked(&document, &shipped, &Env::none()) {
            panic!("{} ships a prompt it would refuse: {refused}", key.name());
        }
    }
}

/// A placeholder is filled once, where the template has it: text a person or
/// a Drone wrote that happens to spell a placeholder is left as written.
#[test]
fn a_placeholder_is_filled_once_and_never_inside_what_filled_another() {
    let filled = crate::prompts::fill(
        "at {root}: {asked} {other}",
        &[("root", "{asked}"), ("asked", "why?")],
    );
    assert_eq!(filled, "at {asked}: why? {other}");
}

/// A Judge prompt saved in settings.json reaches the words `verification` lays
/// its briefs in, and every Judge piece has a key: Fleet owns the override,
/// and that crate never reads a setting.
#[test]
fn a_saved_judge_prompt_is_the_wording_verification_is_handed() {
    use config::settings::{Resolved, Saved};
    let key = keys::PROMPT_JUDGE_OPENING.name().to_string();
    let document = BTreeMap::from([(key, settings_file::SettingValue::Text("Judge it.".into()))]);
    let saved: Saved = checked(&document, &supplied(), &Env::none()).expect("taken");
    let resolved = Resolved::new(saved, supplied(), Env::none());
    let wording = crate::prompts::Prompts::shipped()
        .overlaid_by(&resolved)
        .wording();
    assert_eq!(
        wording.get(verification::pieces::JUDGE_OPENING),
        "Judge it."
    );
    assert_eq!(
        wording.get(verification::pieces::JUDGE_ANSWER),
        verification::pieces::JUDGE_ANSWER.shipped,
        "a piece nobody saved is what ships"
    );
}

/// No shipped prompt carries a run of spaces inside a sentence: the reformat
/// that left fourteen of them between two words of `prompts.crossingOvertaken`
/// is not repeated. Spaces aligning a column, as `Expected   ` does, are laid
/// on purpose and stay.
#[test]
fn no_shipped_prompt_carries_a_run_of_spaces_inside_a_sentence() {
    for key in keys::prompt_keys() {
        let text = crate::prompts::Prompts::shipped().get(key).to_string();
        let chars: Vec<char> = text.chars().collect();
        let mut at = 0;
        while at < chars.len() {
            let run = chars[at..].iter().take_while(|one| **one == ' ').count();
            if run >= 3 && at > 0 && at + run < chars.len() {
                let (before, after) = (chars[at - 1], chars[at + run]);
                assert!(
                    !(before.is_lowercase() && after.is_lowercase()),
                    "{} has {run} spaces between two words at character {at}",
                    key.name()
                );
            }
            at += run.max(1);
        }
    }
}

/// A saved injected-turn prompt is what the next turn says, on both sides of
/// the seam: Fleet's own poke, and the gate's outcome turn that `verification`
/// lays in the wording Fleet hands it.
#[test]
fn a_saved_turn_prompt_is_what_the_next_turn_says() {
    use config::settings::{Resolved, Saved};
    let document = BTreeMap::from([
        (
            keys::PROMPT_POKE_QUIET.name().to_string(),
            settings_file::SettingValue::Text("Quiet for {quiet}.".into()),
        ),
        (
            keys::PROMPT_OUTCOME_LAST.name().to_string(),
            settings_file::SettingValue::Text("Done. Stop.".into()),
        ),
    ]);
    let saved: Saved = checked(&document, &supplied(), &Env::none()).expect("taken");
    let prompts = crate::prompts::Prompts::shipped().overlaid_by(&Resolved::new(
        saved,
        supplied(),
        Env::none(),
    ));
    let poke = crate::silence::Poke::after(&prompts, std::time::Duration::from_secs(180));
    assert_eq!(poke.text(), "Quiet for 3 minutes.");
    let wording = prompts.wording();
    assert_eq!(
        wording.get(verification::pieces::OUTCOME_LAST),
        "Done. Stop."
    );
    assert_eq!(
        wording.get(verification::pieces::OUTCOME_GO_ON),
        verification::pieces::OUTCOME_GO_ON.shipped
    );
}
