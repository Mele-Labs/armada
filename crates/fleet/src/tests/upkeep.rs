//! The librarian's free upkeep part, written before the code: **a feature whose code is gone, or that nothing has touched
//! for months, carries a Retire proposal with its evidence; one whose paths never had a file is Planned; a linked doc that
//! is no longer a file of the repository is `gone`.** Fleet only proposes, and the owner's answer is `answer_retire`. Each
//! test lays a repository out under the rig's root, dates its commits from the rig's clock, and presses the librarian over it.
//! `docs/concepts/feature-catalog.md`, *Librarian*.

use std::process::Command;

use api::Features;
use ipc::{AnswerRetire, AreaSeed, FeatureRow, FeatureSeed, LibrarianStepState, RetireWhy, SettingValue, UpsertFeatures};
use store::{feature_link_kind, KeptFeatureLink};

use super::doc_drift::{commit, git, write};
use super::librarian::{libraried, pressed, rig, set, step, steps_listed, steps_told};
use super::session_host::Rig;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

fn now_ms(rig: &Rig) -> i64 {
    rig.fleet.now().epoch_millis().expect("a clock")
}

/// A commit of `paths` whose author and committer date is `days_ago` days before the rig's clock.
fn commit_aged(rig: &Rig, paths: &[&str], message: &str, days_ago: i64) {
    let secs = (now_ms(rig) - days_ago * DAY_MS) / 1000;
    let mut add = vec!["add", "-A", "--"];
    add.extend(paths);
    git(rig.root.as_str(), &add);
    let out = Command::new("git")
        .current_dir(rig.root.as_str())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_AUTHOR_DATE", format!("{secs} +0000"))
        .env("GIT_COMMITTER_DATE", format!("{secs} +0000"))
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false", "commit", "-q", "-m", message])
        .output()
        .expect("git ran");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

fn head_short(rig: &Rig) -> String {
    let out = Command::new("git").current_dir(rig.root.as_str()).args(["rev-parse", "--short=7", "HEAD"]).output().expect("git ran");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn init(rig: &Rig) {
    git(rig.root.as_str(), &["init", "-q", "-b", "main"]);
}

async fn only(rig: &Rig, part: &str) {
    set(rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text(part.into())])).await;
}

/// A feature in the project's area, with `paths`.
async fn feature_kept(rig: &Rig, project: &str, slug: &str, paths: &[&str]) -> String {
    let upsert = UpsertFeatures {
        project: project.to_string(),
        areas: vec![AreaSeed { slug: "takes".into(), title: "Takes".into() }],
        features: vec![FeatureSeed {
            slug: slug.into(),
            area: "takes".into(),
            title: slug.into(),
            summary: "One line.".into(),
            state: None,
            paths: paths.iter().map(|one| one.to_string()).collect(),
            entry_points: Vec::new(),
            manifest_id: None,
        }],
    };
    rig.fleet.upsert_features(None, upsert).await.expect("kept");
    rig.fleet.store().lock().await.feature_named(project, slug).expect("read").expect("held").id
}

/// A link of `kind` to `target` for `feature`, kept `days_ago` days before the rig's clock.
async fn link_aged(rig: &Rig, feature: &str, kind: &str, target: &str, gist: &str, days_ago: i64) {
    let at = crate::clock::rfc3339_utc(now_ms(rig) - days_ago * DAY_MS);
    let link = KeptFeatureLink {
        feature_id: feature.to_string(),
        kind: kind.to_string(),
        target: target.to_string(),
        job: "j1".to_string(),
        title: String::new(),
        gist: gist.to_string(),
        gist_by: String::new(),
        state: String::new(),
        via: "read".to_string(),
        at,
    };
    rig.fleet.store().lock().await.put_feature_link(&link).expect("kept");
}

async fn row(rig: &Rig, project: &str, slug: &str) -> Option<FeatureRow> {
    let library = rig.fleet.list_features(None, Some(project.to_string())).await.expect("listed");
    library.areas.into_iter().flat_map(|area| area.features).find(|one| one.slug == slug)
}

async fn answered(rig: &Rig, project: &str, slug: &str, retire: bool) -> Result<ipc::RetireAnswered, api::Refusal> {
    rig.fleet.answer_retire(None, AnswerRetire { project: project.to_string(), slug: slug.to_string(), retire }).await
}

/// A repository where the voting feature's one file was committed and then deleted, so its paths match nothing now.
fn voting_code_deleted(rig: &Rig) {
    let root = rig.root.as_str();
    init(rig);
    write(root, "README.md", "A repository.\n");
    write(root, "crates/pages/voting/lib.rs", "pub fn vote() {}\n");
    commit(root, &["README.md", "crates/pages/voting/lib.rs"], "voting");
    git(root, &["rm", "-q", "-r", "crates/pages/voting"]);
    commit_aged(rig, &[], "voting is removed", 0);
}

#[tokio::test]
async fn a_feature_whose_files_were_deleted_is_proposed_for_retire_with_the_last_commit_in_its_evidence() {
    let rig = rig();
    let (project, _) = libraried(&rig).await;
    voting_code_deleted(&rig);
    only(&rig, "retire").await;
    let mut watching = rig.fleet.events().subscribe();

    let ran = pressed(&rig, &project).await;
    assert_eq!(ran.ran, [ipc::LibrarianPart::Retire]);
    let voting = row(&rig, &project, "voting-on-takes").await.expect("still listed: Fleet only proposes");
    let proposal = voting.retire.expect("a proposal");
    assert_eq!(proposal.why, RetireWhy::CodeGone);
    assert!(proposal.evidence.starts_with(&format!("Its paths match no file since {}, ", head_short(&rig))), "{}", proposal.evidence);
    assert!(!voting.planned, "it had files once");
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[3], step("check_upkeep", LibrarianStepState::Done, "1 to retire, nothing planned, no dead links"));
}

#[tokio::test]
async fn a_feature_with_files_nothing_touched_for_180_days_and_no_job_on_is_unused_and_one_with_a_recent_job_link_is_not() {
    let rig = rig();
    let (project, voting) = libraried(&rig).await;
    let ranking = feature_kept(&rig, &project, "ranking", &["crates/pages/ranking"]).await;
    let scoring = feature_kept(&rig, &project, "scoring", &["crates/pages/scoring"]).await;
    let sorting = feature_kept(&rig, &project, "sorting", &["crates/pages/sorting"]).await;
    let root = rig.root.as_str();
    init(&rig);
    for name in ["voting", "ranking", "sorting"] {
        write(root, &format!("crates/pages/{name}/lib.rs"), "pub fn f() {}\n");
    }
    commit_aged(&rig, &["crates"], "three old features", 400);
    write(root, "crates/pages/scoring/lib.rs", "pub fn f() {}\n");
    commit_aged(&rig, &["crates"], "a recent one", 5);
    // `voting` has no Job at all, `ranking` had one 10 days ago, `sorting` had one 300 days ago, `scoring` is recent in git.
    link_aged(&rig, &ranking, feature_link_kind::JOB, "j1", "", 10).await;
    link_aged(&rig, &sorting, feature_link_kind::JOB, "j2", "", 300).await;
    link_aged(&rig, &scoring, feature_link_kind::JOB, "j3", "", 400).await;
    only(&rig, "retire").await;

    pressed(&rig, &project).await;
    let voting_row = row(&rig, &project, "voting-on-takes").await.expect("listed");
    let proposal = voting_row.retire.expect("unused");
    assert_eq!(proposal.why, RetireWhy::Unused);
    assert!(proposal.evidence.starts_with("No change to its files since "), "{}", proposal.evidence);
    assert!(proposal.evidence.ends_with("; no Job has been on it"), "{}", proposal.evidence);
    let _ = voting;
    assert_eq!(row(&rig, &project, "ranking").await.expect("listed").retire, None, "a Job linked 10 days ago");
    assert_eq!(row(&rig, &project, "scoring").await.expect("listed").retire, None, "its files changed 5 days ago");
    let sorting_row = row(&rig, &project, "sorting").await.expect("listed").retire.expect("unused");
    assert!(sorting_row.evidence.contains("; no Job on it since "), "{}", sorting_row.evidence);
}

#[tokio::test]
async fn a_feature_whose_paths_never_had_files_is_planned_and_stops_being_planned_once_a_file_appears() {
    let rig = rig();
    let (project, _) = libraried(&rig).await;
    let root = rig.root.as_str();
    init(&rig);
    write(root, "README.md", "A repository.\n");
    commit(root, &["README.md"], "a start");
    only(&rig, "retire").await;
    let mut watching = rig.fleet.events().subscribe();

    pressed(&rig, &project).await;
    let planned = row(&rig, &project, "voting-on-takes").await.expect("listed");
    assert_eq!((planned.planned, planned.retire), (true, None), "nothing to retire: it was never written");
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[3], step("check_upkeep", LibrarianStepState::Done, "Nothing to retire, 1 planned, no dead links"));

    write(root, "crates/pages/voting/lib.rs", "pub fn vote() {}\n");
    commit(root, &["crates/pages/voting/lib.rs"], "its first code");
    let mut watching = rig.fleet.events().subscribe();
    pressed(&rig, &project).await;
    let normal = row(&rig, &project, "voting-on-takes").await.expect("listed");
    assert_eq!((normal.planned, normal.retire), (false, None), "a normal feature from its first file");
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[3], step("check_upkeep", LibrarianStepState::Done, "Nothing to retire, nothing planned, no dead links"));
}

#[tokio::test]
async fn keep_holds_on_a_rerun_with_the_same_evidence_and_lifts_when_the_evidence_changes() {
    let rig = rig();
    let (project, _) = libraried(&rig).await;
    voting_code_deleted(&rig);
    only(&rig, "retire").await;
    pressed(&rig, &project).await;
    let first = row(&rig, &project, "voting-on-takes").await.expect("listed").retire.expect("proposed");

    let kept = answered(&rig, &project, "voting-on-takes", false).await.expect("answered");
    assert!(!kept.retired);
    assert_eq!(row(&rig, &project, "voting-on-takes").await.expect("still listed").retire, None, "kept, so quiet");

    pressed(&rig, &project).await;
    assert_eq!(row(&rig, &project, "voting-on-takes").await.expect("listed").retire, None, "the same evidence: the Keep holds");

    // The code comes back and goes again: the last commit that touched its paths is a newer one.
    let root = rig.root.as_str();
    write(root, "crates/pages/voting/lib.rs", "pub fn vote_again() {}\n");
    commit(root, &["crates/pages/voting/lib.rs"], "voting again");
    git(root, &["rm", "-q", "-r", "crates/pages/voting"]);
    commit_aged(&rig, &[], "voting is removed again", 0);
    pressed(&rig, &project).await;
    let again = row(&rig, &project, "voting-on-takes").await.expect("listed").retire.expect("proposed again on other evidence");
    assert_ne!(again.evidence, first.evidence);
    assert!(again.evidence.contains(&head_short(&rig)), "{}", again.evidence);
}

#[tokio::test]
async fn retire_removes_the_feature_from_the_catalog_and_keeps_its_links_and_only_a_proposed_one_is_answered() {
    let rig = rig();
    let (project, voting) = libraried(&rig).await;
    let other = feature_kept(&rig, &project, "ranking", &["crates/pages/ranking"]).await;
    let _ = other;
    voting_code_deleted(&rig);
    link_aged(&rig, &voting, feature_link_kind::JOB, "j1", "", 3).await;
    only(&rig, "retire").await;
    pressed(&rig, &project).await;

    let refused = answered(&rig, &project, "ranking", true).await.expect_err("planned, never proposed for Retire");
    assert_eq!(refused.error().code, "fleet.retire_not_proposed");
    let a_job = Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()));
    let refused = rig.fleet.answer_retire(a_job, AnswerRetire { project: project.clone(), slug: "voting-on-takes".into(), retire: true }).await.expect_err("a Job's agent does not answer");
    assert_eq!(refused.error().code, "fleet.not_the_operator");
    assert!(row(&rig, &project, "voting-on-takes").await.is_some(), "nothing was kept on a refusal");

    let gone = answered(&rig, &project, "voting-on-takes", true).await.expect("answered");
    assert!(gone.retired);
    assert!(row(&rig, &project, "voting-on-takes").await.is_none(), "out of list_features");
    assert!(row(&rig, &project, "ranking").await.is_some(), "the other stays");
    assert_eq!(rig.fleet.store().lock().await.feature_links(&voting).expect("read").len(), 1, "its links are kept");
    let unknown = answered(&rig, &project, "voting-on-takes", true).await.expect_err("it is gone");
    assert_eq!(unknown.error().code, "fleet.feature_unknown");
}

#[tokio::test]
async fn a_deleted_linked_doc_is_served_gone_with_its_gist_and_an_address_or_a_doc_still_there_is_not() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    let root = rig.root.as_str();
    init(&rig);
    write(root, "docs/guide.md", "A guide.\n");
    write(root, "docs/whole.md", "A whole one.\n");
    commit(root, &["docs/guide.md", "docs/whole.md"], "two docs");
    git(root, &["rm", "-q", "docs/guide.md"]);
    commit_aged(&rig, &[], "the guide goes", 0);
    for (target, gist) in [("docs/guide.md", "Says how to vote."), ("docs/whole.md", "Says it all."), ("https://example.com/spec", "A spec."), ("/nowhere/else/guide.md", "Elsewhere.")] {
        link_aged(&rig, &feature, feature_link_kind::DOC, target, gist, 2).await;
    }
    only(&rig, "links").await;
    let mut watching = rig.fleet.events().subscribe();

    let ran = pressed(&rig, &project).await;
    assert_eq!(ran.ran, [ipc::LibrarianPart::Links]);
    let page = rig.fleet.get_feature_page(None, Some(project.clone()), "voting-on-takes".to_string(), None, false).await.expect("the page");
    let state = |target: &str| page.docs.iter().find(|doc| doc.target == target).map(|doc| (doc.state.clone(), doc.gist.clone())).expect("linked");
    assert_eq!(state("docs/guide.md"), ("gone".to_string(), "Says how to vote.".to_string()), "marked, and the row keeps its gist");
    assert_eq!(state("docs/whole.md").0, "");
    assert_eq!(state("https://example.com/spec").0, "", "an address is never marked gone by this check");
    assert_eq!(state("/nowhere/else/guide.md").0, "", "a file outside the repository is not this check's");
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[3].2, "Nothing to retire, 1 planned, 1 dead link", "the one feature has no files");

    // The doc comes back: the mark clears.
    write(root, "docs/guide.md", "A guide again.\n");
    commit(root, &["docs/guide.md"], "the guide returns");
    pressed(&rig, &project).await;
    let page = rig.fleet.get_feature_page(None, Some(project), "voting-on-takes".to_string(), None, false).await.expect("the page");
    assert!(page.docs.iter().all(|doc| doc.state.is_empty()));
}

#[tokio::test]
async fn the_step_appears_only_where_retire_or_links_is_on_and_says_the_counts() {
    let rig = rig();
    let (project, feature) = libraried(&rig).await;
    voting_code_deleted(&rig);
    link_aged(&rig, &feature, feature_link_kind::DOC, "docs/a.md", "", 1).await;
    link_aged(&rig, &feature, feature_link_kind::DOC, "docs/b.md", "", 1).await;
    link_aged(&rig, &feature, feature_link_kind::DOC, "https://example.com/spec", "", 1).await;

    only(&rig, "floor_plans").await;
    let ids: Vec<String> = steps_listed(&rig, &project).await.into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids, ["read_merges", "read_through"], "neither part is on");

    for part in ["retire", "links"] {
        only(&rig, part).await;
        let listed = rig.fleet.list_features(None, Some(project.clone())).await.expect("listed").librarian.steps;
        let ids: Vec<&str> = listed.iter().map(|one| one.id.as_str()).collect();
        assert_eq!(ids, ["read_merges", "check_upkeep", "read_through"], "{part} alone puts it in");
        assert_eq!(
            (listed[1].title.as_str(), listed[1].detail.as_str(), listed[1].model),
            ("Check features and links against the code", "The 1 feature and 2 linked docs, against the code at HEAD", false)
        );
    }

    set(&rig, "features.librarianParts", SettingValue::List(["links", "retire"].map(|one| SettingValue::Text(one.into())).to_vec())).await;
    let ids: Vec<String> = steps_listed(&rig, &project).await.into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids.iter().filter(|id| *id == "check_upkeep").count(), 1, "both parts, one step");

    let mut watching = rig.fleet.events().subscribe();
    let ran = pressed(&rig, &project).await;
    assert_eq!(ran.ran, [ipc::LibrarianPart::Links, ipc::LibrarianPart::Retire]);
    let told = steps_told(&mut watching, &project).await;
    assert_eq!(told[2], step("check_upkeep", LibrarianStepState::Running, ""));
    assert_eq!(told[3], step("check_upkeep", LibrarianStepState::Done, "1 to retire, nothing planned, 2 dead links"));
}
