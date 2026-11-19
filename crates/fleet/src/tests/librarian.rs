//! The librarian, written before the code: **it runs on a press, and Fleet never starts a run by itself while the setting says
//! `on_press`, which is what ships.** `docs/concepts/feature-catalog.md`, *Librarian*.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::{Features, Projects, Settings};
use ipc::{AreaSeed, FeatureSeed, LibrarianPart, NewProject, ProjectMember, RunLibrarian, SaveSettings, SettingValue, SinceYouLooked, UpsertFeatures};
use store::{feature_link_kind, KeptFeatureLink};

use testkit::FakeJudge;

use super::peer::Placing;
use super::session_host::{rig_fitted, rig_placing, Rig};

fn rig() -> Rig {
    rig_placing(Some(Placing::nothing()))
}

fn a_job() -> Option<api::Caller> {
    Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()))
}

/// A project holding one feature, and that feature's id.
async fn libraried(rig: &Rig) -> (String, String) {
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
        state: String::new(),
        via: "detector".to_string(),
        at: at.to_string(),
    };
    rig.fleet.store().lock().await.put_feature_link(&link).expect("kept");
}

async fn set(rig: &Rig, key: &str, value: SettingValue) {
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
    set(&rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("links".into())])).await;

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
fn rig_saying(answer: &str) -> (Rig, Arc<FakeJudge>) {
    let judge = Arc::new(FakeJudge::saying(answer));
    let planted = Arc::clone(&judge);
    let rig = rig_fitted(move |fittings| {
        fittings.peers = Placing::nothing();
        fittings.judge = planted;
    });
    (rig, judge)
}

async fn pressed(rig: &Rig, project: &str) -> ipc::LibrarianRan {
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
    assert_eq!(ran.ran, [LibrarianPart::SinceYouLooked]);
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
    assert_eq!(ran.ran, [LibrarianPart::SinceYouLooked], "the part read the facts and found none");
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
    assert_eq!((ran.ran, ran.skipped_for_share), (Vec::new(), vec![LibrarianPart::SinceYouLooked]));
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
    set(&rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("links".into())])).await;

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
