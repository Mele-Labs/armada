//! The librarian, written before the code: **it runs on a press, and Fleet never starts a run by itself while the setting says
//! `on_press`, which is what ships.** `docs/concepts/feature-catalog.md`, *Librarian*.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use api::{Features, Projects, Settings};
use ipc::{AreaSeed, FeatureSeed, LibrarianPart, LibrarianStepState, NewProject, ProjectMember, RunLibrarian, SaveSettings, SettingValue, SinceYouLooked, StopLibrarian, UpsertFeatures};
use store::{feature_link_kind, KeptFeatureLink};

use testkit::FakeJudge;

use super::peer::Placing;
use super::session_host::{rig_fitted, rig_placing, Rig};

pub(super) fn rig() -> Rig {
    rig_placing(Some(Placing::nothing()))
}

fn a_job() -> Option<api::Caller> {
    Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()))
}

/// A project holding one feature, and that feature's id.
pub(super) async fn libraried(rig: &Rig) -> (String, String) {
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let project = rig.fleet.add_project(NewProject { name: "Armada".to_string(), members, place: None, values: None }).await.expect("added").projects.last().expect("the project").id.clone();
    let upsert = UpsertFeatures {
        project: project.clone(),
        areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }],
        features: vec![FeatureSeed {
            slug: "voting-on-takes".into(),
            area: "takes".into(),
            title: "Voting on takes".into(),
            summary: "One line.".into(),
            state: None,
            paths: vec!["crates/pages/voting".into()],
            entry_points: Vec::new(),
            manifest_id: None,
        }],
    };
    rig.fleet.upsert_features(None, upsert).await.expect("kept");
    let feature = rig.fleet.store().lock().await.feature_named(&project, "voting-on-takes").expect("read").expect("held").id;
    (project, feature)
}

/// Fleet saw pull request `number` merge at `at`, touching the feature.
async fn merged(rig: &Rig, feature: &str, number: u64, at: &str) {
    let link = KeptFeatureLink {
        feature_id: feature.to_string(),
        kind: feature_link_kind::MERGE.to_string(),
        target: number.to_string(),
        job: String::new(),
        title: String::new(),
        gist: String::new(),
        gist_by: String::new(),
        state: String::new(),
        via: "detector".to_string(),
        at: at.to_string(),
    };
    rig.fleet.store().lock().await.put_feature_link(&link).expect("kept");
}

pub(super) async fn set(rig: &Rig, key: &str, value: SettingValue) {
    let changes: BTreeMap<String, Option<SettingValue>> = [(key.to_string(), Some(value))].into_iter().collect();
    rig.fleet.save_settings(SaveSettings { changes }).await.expect("saved");
}

async fn runs_kept(rig: &Rig, project: &str) -> u64 {
    rig.fleet.store().lock().await.librarian_run_count(project).expect("read")
}

#[tokio::test]
async fn a_press_runs_the_librarian_records_the_run_and_the_listing_says_what_merged_since() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    merged(&rig, &feature, 3860, "2026-11-17T10:00:00.000Z").await;
    merged(&rig, &feature, 3866, "2026-11-18T08:00:00.000Z").await;
    // A part that is not built, so this run is the frame alone. The paragraph part has tests of its own.
    set(&rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("floor_plans".into())])).await;

    let before = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian;
    assert_eq!((before.last_run, before.read_through), (None, None), "never run");
    assert_eq!(before.merged_since, [3860, 3866], "every merge Fleet kept, oldest first, is unread");

    let ran = rig.fleet.run_librarian(None, RunLibrarian { project: project.clone() }).await.expect("ran");
    assert_eq!(ran.read_through, Some(3866), "it read through the newest merge");
    assert!(ran.ran.is_empty(), "the one part the setting names is not built, so none ran");

    let now = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian;
    assert_eq!(now.last_run, Some(ran.ran_at.clone()));
    assert_eq!((now.read_through, now.merged_since.clone()), (Some(3866), Vec::<u64>::new()));

    merged(&rig, &feature, 3867, "2026-11-19T08:00:00.000Z").await;
    merged(&rig, &feature, 3867, "2026-11-19T08:00:00.000Z").await;
    let behind = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian;
    assert_eq!((behind.read_through, behind.merged_since), (Some(3866), vec![3867]), "behind by the one merge since, said once");

    let kept = rig.fleet.store().lock().await.librarian_last_run(&project).expect("read").expect("kept");
    assert_eq!((kept.by.as_str(), kept.through_pr), ("press", 3866));
}

#[tokio::test]
async fn only_bridge_or_the_operator_presses_and_a_project_fleet_lacks_is_refused() {
    let rig = rig();
    let (project, _) = libraried(&rig).await;
    let refused = rig.fleet.run_librarian(a_job(), RunLibrarian { project: project.clone() }).await.expect_err("a Job's agent does not press");
    assert_eq!(refused.error().code, "fleet.not_the_operator");
    let refused = rig.fleet.run_librarian(None, RunLibrarian { project: "nobody".into() }).await.expect_err("no such project");
    assert_eq!(refused.error().code, "fleet.feature_project");
    assert_eq!(runs_kept(&rig, &project).await, 0, "nothing was kept");
}

#[tokio::test]
async fn fleet_starts_no_run_by_itself_while_runs_is_on_press_which_is_what_ships() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    assert_eq!(rig.fleet.librarian_set().runs, crate::feature_library::librarian::Runs::OnPress, "it ships on press");
    merged(&rig, &feature, 3866, "2026-11-18T08:00:00.000Z").await;
    for _ in 0..3 {
        rig.fleet.librarians_due().await;
    }
    assert_eq!(runs_kept(&rig, &project).await, 0, "a merge it has not read through, and no run: only a press starts one");

    // The same project, merge and tick start a run under the other words, so the line above is the setting holding it back.
    set(&rig, "features.librarianRuns", SettingValue::Text("after_merges".into())).await;
    rig.fleet.librarians_due().await;
    assert_eq!(runs_kept(&rig, &project).await, 1, "after merges, the tick runs it");

    set(&rig, "features.librarianRuns", SettingValue::Text("on_press".into())).await;
    merged(&rig, &feature, 3867, "2026-11-19T08:00:00.000Z").await;
    rig.fleet.librarians_due().await;
    assert_eq!(runs_kept(&rig, &project).await, 1, "back on press, a new merge starts nothing");
}

#[tokio::test]
async fn after_merges_runs_once_for_the_merges_it_has_not_read_and_nightly_once_a_day() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    set(&rig, "features.librarianRuns", SettingValue::Text("after_merges".into())).await;
    rig.fleet.librarians_due().await;
    assert_eq!(runs_kept(&rig, &project).await, 0, "no merge to read");
    merged(&rig, &feature, 3866, "2026-11-18T08:00:00.000Z").await;
    rig.fleet.librarians_due().await;
    rig.fleet.librarians_due().await;
    assert_eq!(runs_kept(&rig, &project).await, 1, "the merge is read through once");
    assert_eq!(rig.fleet.store().lock().await.librarian_last_run(&project).expect("read").expect("kept").by, "after_merges");

    set(&rig, "features.librarianRuns", SettingValue::Text("nightly".into())).await;
    rig.fleet.librarians_due().await;
    assert_eq!(runs_kept(&rig, &project).await, 1, "ran a moment ago, so not due tonight");
}

#[tokio::test]
async fn the_parts_a_run_does_are_the_ones_the_setting_names_and_a_name_that_is_no_part_is_left_out() {
    let rig = rig();
    assert_eq!(rig.fleet.librarian_set().parts, LibrarianPart::ALL, "every part is on as it ships");
    assert_eq!(rig.fleet.librarian_set().model_share, 20);
    let names = SettingValue::List(["gists", "bogus", "links"].map(|one| SettingValue::Text(one.into())).to_vec());
    set(&rig, "features.librarianParts", names).await;
    assert_eq!(rig.fleet.librarian_set().parts, [LibrarianPart::Links, LibrarianPart::Gists], "in the order they run, the stranger dropped");
    set(&rig, "features.librarianModelShare", SettingValue::Integer(5)).await;
    assert_eq!(rig.fleet.librarian_set().model_share, 5);
}

#[test]
fn the_setting_names_every_part_the_wire_does_in_the_same_order() {
    let wire: Vec<&str> = LibrarianPart::ALL.iter().map(|part| part.name()).collect();
    assert_eq!(wire, config::settings::LIBRARIAN_PART_NAMES);
}

// The since_you_looked part: one paragraph from the facts, spent from the night's share.

const WORDS: &str = "Voting on takes changed with two merged pull requests. One feature waits on the owner.";

/// A rig whose small model answers `answer`, and the model that was asked.
pub(super) fn rig_saying(answer: &str) -> (Rig, Arc<FakeJudge>) {
    rig_judged(FakeJudge::saying(answer))
}

fn rig_judged(judge: FakeJudge) -> (Rig, Arc<FakeJudge>) {
    let judge = Arc::new(judge);
    let planted = Arc::clone(&judge);
    let rig = rig_fitted(move |fittings| {
        fittings.peers = Placing::nothing();
        fittings.judge = planted;
    });
    (rig, judge)
}

pub(super) async fn pressed(rig: &Rig, project: &str) -> ipc::LibrarianRan {
    rig.fleet.run_librarian(None, RunLibrarian { project: project.to_string() }).await.expect("ran")
}

async fn the_page(rig: &Rig, project: &str, since: &str) -> SinceYouLooked {
    rig.fleet.get_since_you_looked(None, Some(project.to_string()), since.to_string()).await.expect("read")
}

#[tokio::test]
async fn a_press_with_changes_writes_a_paragraph_and_the_page_serves_it_to_one_who_looked_before_the_run() {
    let (rig, judge) = rig_saying(WORDS);
    let (project, feature) = libraried(&rig).await;
    merged(&rig, &feature, 3866, "2026-08-26T08:00:00.000Z").await;

    let ran = pressed(&rig, &project).await;
    assert_eq!(ran.ran, [LibrarianPart::Drift, LibrarianPart::SinceYouLooked]);
    assert!(ran.skipped_for_share.is_empty());
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one call, for the one part");
    assert!(asked[0].contains("Voting on takes") && asked[0].contains("At most three short sentences"), "{}", asked[0]);

    assert_eq!(the_page(&rig, &project, "2026-08-25T00:00:00Z").await.paragraph, WORDS, "he looked before the run");
    assert_eq!(the_page(&rig, &project, "2026-08-27T00:00:00Z").await.paragraph, "", "he looked after the run, so it says nothing new");
}

#[tokio::test]
async fn a_press_with_nothing_new_since_the_last_run_makes_no_call_and_writes_no_paragraph() {
    let (rig, judge) = rig_saying(WORDS);
    let (project, _) = libraried(&rig).await;
    pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 1, "the first run reads the feature as new");

    let ran = pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 1, "nothing moved since the last run, so nothing was asked");
    assert_eq!(ran.ran, [LibrarianPart::Drift, LibrarianPart::SinceYouLooked], "the part read the facts and found none");
    let kept = rig.fleet.store().lock().await.librarian_last_run(&project).expect("read").expect("kept");
    assert_eq!((kept.paragraph.as_str(), kept.model_calls), ("", 0));
}

#[tokio::test]
async fn a_spent_share_skips_the_part_makes_no_call_and_the_run_says_so() {
    let (rig, judge) = rig_saying(WORDS);
    let (project, feature) = libraried(&rig).await;
    set(&rig, "features.librarianModelShare", SettingValue::Integer(1)).await;
    pressed(&rig, &project).await;
    merged(&rig, &feature, 3866, "2026-08-26T23:00:00.000Z").await;

    let ran = pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 1, "the night's one call was spent on the first press");
    assert_eq!((ran.ran, ran.skipped_for_share), (vec![LibrarianPart::Drift], vec![LibrarianPart::SinceYouLooked]));
    let kept = rig.fleet.store().lock().await.librarian_last_run(&project).expect("read").expect("kept");
    assert_eq!(kept.skipped, ["since_you_looked"], "the record says why");

}

#[tokio::test]
async fn a_share_of_none_spends_nothing() {
    let (rig, judge) = rig_saying(WORDS);
    let (project, _) = libraried(&rig).await;
    set(&rig, "features.librarianModelShare", SettingValue::Integer(0)).await;
    assert_eq!(pressed(&rig, &project).await.skipped_for_share, [LibrarianPart::SinceYouLooked]);
    assert!(judge.asked().is_empty());
}

#[tokio::test]
async fn the_part_off_in_the_setting_makes_no_call() {
    let (rig, judge) = rig_saying(WORDS);
    let (project, feature) = libraried(&rig).await;
    merged(&rig, &feature, 3866, "2026-08-26T08:00:00.000Z").await;
    set(&rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("floor_plans".into())])).await;

    let ran = pressed(&rig, &project).await;
    assert!(judge.asked().is_empty(), "the part is off, so no words are asked for");
    assert_eq!((ran.ran, ran.skipped_for_share), (Vec::new(), Vec::new()));
    assert_eq!(the_page(&rig, &project, "2026-08-25T00:00:00Z").await.paragraph, "");
}

#[test]
fn the_answer_is_kept_as_three_sentences_at_most_with_no_dash_and_an_empty_one_is_nothing() {
    use crate::feature_library::since_paragraph::paragraph_of;
    let kept = paragraph_of("\"One feature is new. Two changed \u{2014} both in votes. A third waits on you! A fourth.\"").expect("kept");
    assert_eq!(kept, "One feature is new. Two changed, both in votes. A third waits on you!");
    assert!(!kept.contains(['\u{2014}', '\u{2013}']));
    assert_eq!(paragraph_of("  \n "), None);
}

// The steps a run takes, listed before a press and told as each one moves. The sketch "Catch up shows its steps".

/// The steps the listing names now: `(id, model, detail)`.
pub(super) async fn steps_listed(rig: &Rig, project: &str) -> Vec<(String, bool, String)> {
    let state = rig.fleet.list_features(None, Some(project.to_string())).await.expect("listed").librarian;
    state.steps.into_iter().map(|step| (step.id, step.model, step.detail)).collect()
}

/// What `librarian.step` told since the last drain: `(step, state, said)`, each checked against `project`.
pub(super) async fn steps_told(watching: &mut api::Subscription, project: &str) -> Vec<(String, LibrarianStepState, String)> {
    let mut told = Vec::new();
    while let Ok(Some(api::Next::Send(delivered))) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        if let ipc::Event::LibrarianStep(moved) = delivered.event {
            assert_eq!(moved.project, project);
            told.push((moved.step, moved.state, moved.said));
        }
    }
    told
}

pub(super) fn step(id: &str, state: LibrarianStepState, said: &str) -> (String, LibrarianStepState, String) {
    (id.to_string(), state, said.to_string())
}

/// A section a Job wrote of the feature at `at`.
async fn section_written(rig: &Rig, feature: &str, at: &str) {
    let kept = store::KeptSection { feature_id: feature.to_string(), section: "test".into(), text: "Run it.".into(), job: "j1".into(), written_at: at.to_string(), at_commit: String::new() };
    rig.fleet.store().lock().await.put_feature_section(&kept).expect("kept");
}

#[tokio::test]
async fn the_listing_names_the_steps_a_run_would_take_each_free_or_model_with_the_merges_since_and_the_calls_left() {
    let (rig, _) = rig_saying(WORDS);
    let (project, feature) = libraried(&rig).await;
    for number in [3862, 3863, 3864] {
        merged(&rig, &feature, number, &format!("2026-08-26T0{}:00:00.000Z", number - 3860)).await;
    }
    assert_eq!(
        steps_listed(&rig, &project).await,
        [
            ("read_merges".to_string(), false, "Pull requests #3862, #3863 and #3864, and the sections their Jobs wrote".to_string()),
            ("check_upkeep".to_string(), false, "The 1 feature and 0 linked docs, against the code at HEAD".to_string()),
            ("check_drift".to_string(), false, "No docs are linked to features yet".to_string()),
            ("since_you_looked".to_string(), true, "One model call, skipped if nothing is new. 20 of 20 calls left tonight".to_string()),
            ("read_through".to_string(), false, "Read through #3864".to_string()),
        ]
    );

    pressed(&rig, &project).await;
    let after = steps_listed(&rig, &project).await;
    assert_eq!(after[0].2, "Nothing merged since #3864", "the run read them");
    assert!(after[3].2.ends_with("19 of 20 calls left tonight"), "{}", after[3].2);

    set(&rig, "features.librarianModelShare", SettingValue::Integer(1)).await;
    assert_eq!(steps_listed(&rig, &project).await[3].2, "Skipped: tonight's 1 call is spent");

    // A part with no work behind it has no step, and neither has what a page reads live.
    set(&rig, "features.librarianParts", SettingValue::List(["floor_plans", "health"].map(|one| SettingValue::Text(one.into())).to_vec())).await;
    let ids: Vec<String> = steps_listed(&rig, &project).await.into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids, ["read_merges", "read_through"]);
}

#[tokio::test]
async fn a_never_run_project_with_nothing_merged_says_so_in_its_steps() {
    let (rig, _) = rig_saying(WORDS);
    let (project, _) = libraried(&rig).await;
    let listed = steps_listed(&rig, &project).await;
    assert_eq!((listed[0].2.as_str(), listed[4].2.as_str()), ("Nothing merged yet", "Nothing merged to read through"));
}

#[tokio::test]
async fn a_press_tells_each_step_running_then_done_in_order_with_a_line_of_what_it_did() {
    let (rig, _) = rig_saying(WORDS);
    let (project, feature) = libraried(&rig).await;
    merged(&rig, &feature, 3866, "2026-08-26T08:00:00.000Z").await;
    section_written(&rig, &feature, "2026-08-26T09:00:00.000Z").await;
    let mut watching = rig.fleet.events().subscribe();

    let ran = pressed(&rig, &project).await;
    assert!(!ran.stopped);
    assert_eq!(
        steps_told(&mut watching, &project).await,
        [
            step("read_merges", LibrarianStepState::Running, ""),
            step("read_merges", LibrarianStepState::Done, "1 merge, 1 section"),
            step("check_upkeep", LibrarianStepState::Running, ""),
            step("check_upkeep", LibrarianStepState::Skipped, "No repository could be read, so nothing was marked"),
            step("check_drift", LibrarianStepState::Running, ""),
            step("check_drift", LibrarianStepState::Done, "No doc names what the code no longer has"),
            step("since_you_looked", LibrarianStepState::Running, ""),
            step("since_you_looked", LibrarianStepState::Done, "Wrote 2 sentences, 1 of 20 calls tonight"),
            step("read_through", LibrarianStepState::Running, ""),
            step("read_through", LibrarianStepState::Done, "Read through #3866"),
        ]
    );
}

#[tokio::test]
async fn every_step_of_one_run_names_the_run_the_answer_names() {
    let (rig, _) = rig_saying(WORDS);
    let (project, _) = libraried(&rig).await;
    let mut watching = rig.fleet.events().subscribe();
    let ran = pressed(&rig, &project).await;
    let mut runs = Vec::new();
    while let Ok(Some(api::Next::Send(delivered))) = tokio::time::timeout(Duration::from_millis(300), watching.next()).await {
        if let ipc::Event::LibrarianStep(moved) = delivered.event {
            runs.push(moved.run);
        }
    }
    assert_eq!(runs.len(), 10);
    assert!(runs.iter().all(|run| *run == ran.ran_at), "{runs:?}");
}

#[tokio::test]
async fn nothing_new_since_the_last_run_skips_the_model_step_and_says_so() {
    let (rig, judge) = rig_saying(WORDS);
    let (project, _) = libraried(&rig).await;
    pressed(&rig, &project).await;
    let mut watching = rig.fleet.events().subscribe();

    pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 1, "the second press asked nothing");
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[0..8], [
        step("read_merges", LibrarianStepState::Running, ""),
        step("read_merges", LibrarianStepState::Done, "No merges, no sections"),
        step("check_upkeep", LibrarianStepState::Running, ""),
        step("check_upkeep", LibrarianStepState::Skipped, "No repository could be read, so nothing was marked"),
        step("check_drift", LibrarianStepState::Running, ""),
        step("check_drift", LibrarianStepState::Done, "No doc names what the code no longer has"),
        step("since_you_looked", LibrarianStepState::Running, ""),
        step("since_you_looked", LibrarianStepState::Skipped, "Nothing new since the last run, so no call"),
    ]);
    assert_eq!(told[9], step("read_through", LibrarianStepState::Done, "Nothing merged to read through"));
}

#[tokio::test]
async fn a_spent_share_skips_the_model_step_and_says_so() {
    let (rig, judge) = rig_saying(WORDS);
    let (project, feature) = libraried(&rig).await;
    set(&rig, "features.librarianModelShare", SettingValue::Integer(1)).await;
    pressed(&rig, &project).await;
    merged(&rig, &feature, 3866, "2026-08-26T23:00:00.000Z").await;
    let mut watching = rig.fleet.events().subscribe();

    pressed(&rig, &project).await;
    assert_eq!(judge.asked().len(), 1);
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[7], step("since_you_looked", LibrarianStepState::Skipped, "Tonight's 1 call is spent"));
    assert_eq!(told[9], step("read_through", LibrarianStepState::Done, "Read through #3866"), "the free steps still ran");
}

#[tokio::test]
async fn stop_skips_the_steps_after_the_one_running_and_the_run_is_kept_as_stopped() {
    let (rig, judge) = rig_judged(FakeJudge::saying(WORDS).taking(Duration::from_millis(800)));
    let (project, feature) = libraried(&rig).await;
    merged(&rig, &feature, 3866, "2026-08-26T08:00:00.000Z").await;
    let stopped = rig.fleet.stop_librarian(None, StopLibrarian { project: project.clone() }).await.expect("answered");
    assert!(!stopped.stopping, "no run was going");
    let mut watching = rig.fleet.events().subscribe();

    let fleet = Arc::clone(&rig.fleet);
    let pressing = project.clone();
    let run = tokio::spawn(async move { fleet.run_librarian(None, RunLibrarian { project: pressing }).await });
    loop {
        match tokio::time::timeout(Duration::from_secs(5), watching.next()).await.expect("an event") {
            Some(api::Next::Send(delivered)) => match delivered.event {
                ipc::Event::LibrarianStep(moved) if moved.step == "since_you_looked" && moved.state == LibrarianStepState::Running => break,
                _ => {}
            },
            other => panic!("the stream ended: {other:?}"),
        }
    }
    let again = rig.fleet.run_librarian(None, RunLibrarian { project: project.clone() }).await.expect_err("one run at a time");
    assert_eq!(again.error().code, "fleet.librarian_running");
    let a_job = rig.fleet.stop_librarian(a_job(), StopLibrarian { project: project.clone() }).await.expect_err("a Job's agent does not press");
    assert_eq!(a_job.error().code, "fleet.not_the_operator");
    assert!(rig.fleet.stop_librarian(None, StopLibrarian { project: project.clone() }).await.expect("answered").stopping);

    let ran = run.await.expect("joined").expect("ran");
    assert!(ran.stopped);
    assert_eq!((ran.ran, ran.read_through), (vec![LibrarianPart::Drift, LibrarianPart::SinceYouLooked], None), "the step running ended as it would have, and nothing was read through");
    assert_eq!(judge.asked().len(), 1);
    assert_eq!(
        steps_told(&mut watching, &project).await,
        [
            step("since_you_looked", LibrarianStepState::Done, "Wrote 2 sentences, 1 of 20 calls tonight"),
            step("read_through", LibrarianStepState::Skipped, "Stopped"),
        ]
    );
    let kept = rig.fleet.store().lock().await.librarian_last_run(&project).expect("read").expect("kept");
    assert!(kept.stopped, "the record says it was stopped");
    let state = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian;
    assert_eq!((state.read_through, state.merged_since), (None, vec![3866]), "still unread");
    assert_eq!(state.running, None, "a stopped run is no longer going");
    assert!(!rig.fleet.stop_librarian(None, StopLibrarian { project }).await.expect("answered").stopping, "the run is over");
}

#[tokio::test]
async fn a_read_mid_run_names_the_run_going_and_its_steps_so_far_and_none_once_it_ends() {
    let (rig, _) = rig_judged(FakeJudge::saying(WORDS).taking(Duration::from_millis(800)));
    let (project, feature) = libraried(&rig).await;
    merged(&rig, &feature, 3866, "2026-08-26T08:00:00.000Z").await;
    assert_eq!(rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian.running, None, "nothing is going");
    let mut watching = rig.fleet.events().subscribe();

    let fleet = Arc::clone(&rig.fleet);
    let pressing = project.clone();
    let run = tokio::spawn(async move { fleet.run_librarian(None, RunLibrarian { project: pressing }).await });
    loop {
        match tokio::time::timeout(Duration::from_secs(5), watching.next()).await.expect("an event") {
            Some(api::Next::Send(delivered)) => match delivered.event {
                ipc::Event::LibrarianStep(moved) if moved.step == "since_you_looked" && moved.state == LibrarianStepState::Running => break,
                _ => {}
            },
            other => panic!("the stream ended: {other:?}"),
        }
    }
    let going = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian.running.expect("a run is going");
    let so_far: Vec<(String, LibrarianStepState, String)> = going.steps.into_iter().map(|now| (now.step, now.state, now.said)).collect();
    assert_eq!(so_far, [
            step("read_merges", LibrarianStepState::Done, "1 merge, no sections"),
            step("check_upkeep", LibrarianStepState::Skipped, "No repository could be read, so nothing was marked"),
            step("check_drift", LibrarianStepState::Done, "No doc names what the code no longer has"),
            step("since_you_looked", LibrarianStepState::Running, ""),
        ], "read_through is not reached yet");

    let ran = run.await.expect("joined").expect("ran");
    assert_eq!(going.run, ran.ran_at, "it named the run the answer names");
    assert_eq!(rig.fleet.list_features(None, Some(project)).await.expect("listed").librarian.running, None, "the run ended");
}
