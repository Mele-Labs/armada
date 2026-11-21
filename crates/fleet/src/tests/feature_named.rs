//! A Job owes sections only for the features its commits name in a `Feature:` line. Files it changed under another
//! feature link the Job there and owe nothing.

use api::{Features, Sessions};
use ipc::{AllDone, FeatureSectionKind};

use super::agent_starts::{caller, rig};
use super::feature_library::{a_project, an_agent_for, commit_of, feature, named, pushed, refused, seeded, wrote, write_all, VOTING};

const VOTE_FILE: &str = "crates/pages/voting/ranks.rs";
const ACTION_FILE: &str = "crates/pages/actions/menu.rs";

#[tokio::test]
async fn a_job_that_changed_a_features_files_without_naming_it_is_not_held_and_stays_linked() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    wrote(&rig, &id, VOTE_FILE).await;
    pushed(&rig, &id, "820").await;

    assert!(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await.is_ok());
    let store = rig.fleet.store().lock().await;
    assert_eq!(store.feature_files_of_job(id.as_str()).unwrap().len(), 1, "the write is still filed under the feature");
}

#[tokio::test]
async fn a_job_that_named_a_feature_in_a_commit_is_held_until_it_writes_that_features_sections() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    named(&rig, &id, VOTING).await;
    pushed(&rig, &id, "821").await;

    let (code, said) = refused(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await);
    assert_eq!(code, "fleet.feature_sections_owed");
    assert!(said.contains("because your commits named these features") && said.contains("voting-on-takes: test, runs, observe, debt"), "{said}");

    rig.fleet.write_feature_sections(caller(), write_all(&id, VOTING, &FeatureSectionKind::ALL)).await.expect("written");
    assert!(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await.is_ok());
}

#[tokio::test]
async fn files_across_three_features_owe_only_the_one_the_commit_named() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    {
        let mut store = rig.fleet.store().lock().await;
        let ripple = store::KeptFeature { project: project.clone(), added_at: "2000-01-01T00:00:00.000Z".to_string(), ..feature("f-ripple", "ripple", &["crates/ripple"]) };
        store.put_feature(&ripple).expect("kept");
    }
    for file in [VOTE_FILE, ACTION_FILE, "crates/ripple/a.rs"] {
        wrote(&rig, &id, file).await;
    }
    named(&rig, &id, "page-actions").await;
    pushed(&rig, &id, "822").await;

    let (_, said) = refused(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await);
    assert!(said.contains("page-actions: test, runs, observe, debt"), "{said}");
    assert!(!said.contains("voting-on-takes") && !said.contains("ripple"), "changed but not named: {said}");

    rig.fleet.write_feature_sections(caller(), write_all(&id, "page-actions", &FeatureSectionKind::ALL)).await.expect("written");
    assert!(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await.is_ok(), "nothing else is owed");
}

#[tokio::test]
async fn a_commit_link_names_a_feature_too() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    {
        let mut store = rig.fleet.store().lock().await;
        let feature = store.feature_named(&project, VOTING).unwrap().expect("seeded");
        let link = store::KeptFeatureLink {
            feature_id: feature.id,
            kind: store::feature_link_kind::COMMIT.into(),
            target: "abc123".into(),
            job: id.as_str().into(),
            title: String::new(),
            state: String::new(),
            via: "trailer".into(),
            at: rig.fleet.now().as_str().into(),
            gist: String::new(),
            gist_by: String::new(),
        };
        store.put_feature_link(&link).expect("linked");
    }
    pushed(&rig, &id, "823").await;
    let (_, said) = refused(rig.fleet.all_done(caller(), AllDone { session_id: Some(id.clone()), summary: None }).await);
    assert!(said.contains("voting-on-takes"), "{said}");
}

#[tokio::test]
async fn the_gate_keeps_each_name_once_however_many_commits_repeat_it_and_a_commit_naming_none_keeps_nothing() {
    let rig = rig(None);
    let project = a_project(&rig).await;
    rig.fleet.upsert_features(None, seeded(&project)).await.expect("seeded");
    let id = an_agent_for(&rig, &project).await;
    let kept = || async { rig.fleet.store().lock().await.feature_ids_named_by_job(id.as_str()).unwrap() };

    named(&rig, &id, VOTING).await;
    let first = kept().await;
    assert_eq!(first.len(), 1);
    named(&rig, &id, VOTING).await;
    assert_eq!(kept().await, first, "a repeat adds nothing");
    named(&rig, &id, "voting-on-takes, page-actions").await;
    assert_eq!(kept().await.len(), 2, "only the new name is added");

    // A commit with no Feature: line is refused by the gate, and names nothing.
    let refusal = rig.fleet.check_commit_feature(commit_of(&id, r#"git commit -m "Work""#, &[VOTE_FILE])).await.expect("asked");
    assert!(!refusal.allowed);
    assert_eq!(kept().await.len(), 2);

    // Another Job's names are its own.
    let other = an_agent_for(&rig, &project).await;
    assert!(rig.fleet.store().lock().await.feature_ids_named_by_job(other.as_str()).unwrap().is_empty());
}
