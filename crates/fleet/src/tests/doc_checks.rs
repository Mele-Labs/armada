//! The model's check of a feature's docs against its code, written before the code: **only his press of Check runs it**,
//! each doc that is a file of the repository goes to the model with the lines of the feature's code its names point at,
//! each call is spent from the night's share, and nothing is written back. A doc that disagrees is flagged until he
//! dismisses it, which holds until the doc or the code changes, or starts a Job that asks him which is right.
//! `docs/concepts/feature-catalog.md`, *Librarian*.

use std::sync::Arc;

use api::{Features, HostedSessions};
use ipc::{CheckDocs, DocFlag, SettingValue, SessionRow};
use store::{feature_link_kind, KeptFeatureLink};
use testkit::FakeJudge;

use super::doc_drift::{commit, git, write};
use super::librarian::{libraried, rig_judged, rig_saying, set};
use super::session_host::Rig;

const DOC: &str = "docs/takes.md";
const SAYS: &str = "# Takes\n\nA pick is sent with `send_pick`, and ranks are kept for later.\n";
const CODE: &str = "// Sending a pick.\npub fn send_pick() { on_return(); }\n";
const DISAGREES: &str = "DOC: \"ranks are kept for later\"\nCODE: crates/pages/voting/send.rs:2\nWHAT: sends the pick on Return \u{2014} and keeps no rank";

/// A repository at the rig's root with the feature's code and the doc, and the doc linked to the feature by a Job's read,
/// with an address beside it.
async fn laid_out(rig: &Rig) -> String {
    let (project, feature) = libraried(rig).await;
    let root = rig.root.as_str();
    git(root, &["init", "-q", "-b", "main"]);
    write(root, DOC, SAYS);
    write(root, "crates/pages/voting/send.rs", CODE);
    commit(root, &[DOC, "crates/pages/voting/send.rs"], "the feature");
    for (target, minute) in [(DOC, 1), ("https://example.com/spec", 2)] {
        let link = KeptFeatureLink {
            feature_id: feature.clone(),
            kind: feature_link_kind::DOC.to_string(),
            target: target.to_string(),
            job: "j1".to_string(),
            title: String::new(),
            gist: String::new(),
            gist_by: String::new(),
            state: String::new(),
            via: "read".to_string(),
            at: format!("2026-11-19T00:{minute:02}:00.000Z"),
        };
        rig.fleet.store().lock().await.put_feature_link(&link).expect("kept");
    }
    project
}

async fn pressed(rig: &Rig, project: &str) -> Result<ipc::DocsChecked, api::Refusal> {
    rig.fleet.check_docs(None, CheckDocs { project: project.to_string(), slug: "voting-on-takes".into() }).await
}

fn flag(project: &str) -> DocFlag {
    DocFlag { project: project.to_string(), slug: "voting-on-takes".into(), target: DOC.into() }
}

/// What the page carries of the doc's check.
async fn served(rig: &Rig, project: &str) -> Option<ipc::DocCheck> {
    let page = rig.fleet.get_feature_page(None, Some(project.to_string()), "voting-on-takes".to_string(), None, false).await.expect("the page");
    page.docs.into_iter().find(|doc| doc.target == DOC).and_then(|doc| doc.check)
}

#[tokio::test]
async fn a_press_puts_the_doc_to_the_model_with_the_code_its_names_point_at_and_the_page_carries_the_disagreement() {
    let (rig, judge) = rig_saying(DISAGREES);
    let project = laid_out(&rig).await;

    let checked = pressed(&rig, &project).await.expect("checked");
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one call a doc, and the address is not one");
    assert!(asked[0].contains("Document docs/takes.md") && asked[0].contains("ranks are kept for later"), "{}", asked[0]);
    assert!(asked[0].contains("crates/pages/voting/send.rs:2: pub fn send_pick() { on_return(); }"), "the line its name points at: {}", asked[0]);
    assert!(asked[0].contains("Feature: Voting on takes") && asked[0].contains("No advice, no opinion, no dashes"), "{}", asked[0]);
    assert_eq!((checked.calls, checked.checked.len(), checked.unchecked.len()), (1, 1, 0));

    let found = served(&rig, &project).await.expect("checked").disagrees.expect("a disagreement");
    assert_eq!(found.doc_quote, "ranks are kept for later");
    assert_eq!(found.code_at, "crates/pages/voting/send.rs:2");
    assert_eq!(found.code_quote, "sends the pick on Return, and keeps no rank", "on one line, no dash");
    assert_eq!(std::fs::read_to_string(std::path::Path::new(&rig.root).join(DOC)).expect("read"), SAYS, "nothing is written back");
}

#[tokio::test]
async fn a_doc_the_model_finds_agreeing_carries_when_it_was_checked_and_no_disagreement() {
    let (rig, _) = rig_saying("AGREES");
    let project = laid_out(&rig).await;
    pressed(&rig, &project).await.expect("checked");
    let check = served(&rig, &project).await.expect("checked");
    assert_eq!((check.disagrees, check.dismissed), (None, false));
}

#[tokio::test]
async fn a_spent_share_refuses_the_press_in_a_sentence_and_asks_nothing() {
    let (rig, judge) = rig_saying("AGREES");
    let project = laid_out(&rig).await;
    set(&rig, "features.librarianModelShare", SettingValue::Integer(1)).await;
    pressed(&rig, &project).await.expect("the night's one call");
    let refused = pressed(&rig, &project).await.expect_err("the share is spent");
    assert_eq!(refused.error().code, "fleet.librarian_share_spent");
    assert_eq!(refused.error().message, "The librarian's share is spent tonight: 1 of 1 calls.");
    assert_eq!(judge.asked().len(), 1);
}

#[tokio::test]
async fn a_reply_fleet_cannot_read_keeps_nothing_and_still_counts() {
    let (rig, _) = rig_saying("I would rather not say.");
    let project = laid_out(&rig).await;
    let refused = pressed(&rig, &project).await.expect_err("no answer read");
    assert_eq!(refused.error().code, "fleet.doc_check_failed");
    assert!(refused.error().message.contains("1 of 20 calls tonight"), "{}", refused.error().message);
    assert_eq!(served(&rig, &project).await, None);
}

#[tokio::test]
async fn a_dismissal_holds_until_the_doc_changes_and_the_doc_is_not_asked_about_meanwhile() {
    let (rig, judge) = rig_judged(FakeJudge::saying(DISAGREES));
    let project = laid_out(&rig).await;
    pressed(&rig, &project).await.expect("checked");

    rig.fleet.dismiss_doc_flag(None, flag(&project)).await.expect("dismissed");
    assert!(served(&rig, &project).await.expect("checked").dismissed);
    let refused = rig.fleet.dismiss_doc_flag(None, flag(&project)).await.expect_err("nothing stands to dismiss");
    assert_eq!(refused.error().code, "fleet.doc_not_flagged");

    let again = pressed(&rig, &project).await.expect("checked");
    assert_eq!((judge.asked().len(), again.calls), (1, 0), "a dismissed doc that has not changed is not asked about");
    assert!(served(&rig, &project).await.expect("checked").dismissed);

    write(rig.root.as_str(), DOC, &format!("{SAYS}\nA second paragraph.\n"));
    commit(rig.root.as_str(), &[DOC], "the doc changed");
    pressed(&rig, &project).await.expect("checked");
    assert_eq!(judge.asked().len(), 2);
    let check = served(&rig, &project).await.expect("checked");
    assert!(check.disagrees.is_some() && !check.dismissed, "a changed doc is flagged again: {check:?}");
}

#[tokio::test]
async fn start_a_job_starts_one_in_the_project_whose_brief_quotes_both_and_asks_which_is_right() {
    let (rig, _) = rig_saying(DISAGREES);
    let project = laid_out(&rig).await;
    let refused = Arc::clone(&rig.fleet).start_doc_job(None, flag(&project)).await.expect_err("nothing is flagged yet");
    assert_eq!(refused.error().code, "fleet.doc_not_flagged");
    pressed(&rig, &project).await.expect("checked");

    let started = Arc::clone(&rig.fleet).start_doc_job(None, flag(&project)).await.expect("started");
    assert_eq!(started.title, "Doc or code: Voting on takes");
    for said in ["Voting on takes (`voting-on-takes`)", "The doc, docs/takes.md:\n> ranks are kept for later", "The code, crates/pages/voting/send.rs:2:\n> sends the pick on Return", "ask the person which is right before you change anything"] {
        assert!(started.brief.contains(said), "the brief carries {said}: {}", started.brief);
    }
    let record = Arc::clone(&rig.fleet).get_session(started.session_id.clone()).await.expect("kept").session;
    assert_eq!(record.title.as_deref(), Some("Doc or code: Voting on takes"));
    assert_eq!(record.manifest_id.as_ref().map(|id| id.as_str()), Some(rig.manifest.as_str()));
    assert_eq!(started.send_failed, None);
    let rows = rig.rows(&started.session_id).await;
    assert!(rows.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if *text == started.brief)), "the brief is its first message: {rows:#?}");
    assert!(served(&rig, &project).await.expect("checked").disagrees.is_some(), "the flag stays until he dismisses it");
}

#[tokio::test]
async fn a_jobs_agent_cannot_check_dismiss_or_start() {
    let (rig, judge) = rig_saying(DISAGREES);
    let project = laid_out(&rig).await;
    let a_job = || Some(api::Caller::at("127.0.0.1:50000".parse().unwrap()));
    let refused = rig.fleet.check_docs(a_job(), CheckDocs { project: project.clone(), slug: "voting-on-takes".into() }).await.expect_err("the person's press");
    assert_eq!(refused.error().code, "fleet.not_the_operator");
    assert!(judge.asked().is_empty());
    pressed(&rig, &project).await.expect("checked");
    assert_eq!(rig.fleet.dismiss_doc_flag(a_job(), flag(&project)).await.expect_err("his").error().code, "fleet.not_the_operator");
    assert_eq!(Arc::clone(&rig.fleet).start_doc_job(a_job(), flag(&project)).await.expect_err("his").error().code, "fleet.not_the_operator");
}

#[tokio::test]
async fn a_presss_call_leaves_the_librarians_run_that_much_less_of_the_share() {
    let (rig, judge) = rig_saying("AGREES");
    let project = laid_out(&rig).await;
    set(&rig, "features.librarianModelShare", SettingValue::Integer(1)).await;
    set(&rig, "features.librarianParts", SettingValue::List(vec![SettingValue::Text("gists".into())])).await;
    pressed(&rig, &project).await.expect("the night's one call");

    let ran = rig.fleet.run_librarian(None, ipc::RunLibrarian { project: project.clone() }).await.expect("ran");
    assert_eq!(ran.skipped_for_share, [ipc::LibrarianPart::Gists], "the doc lacking a gist waits for tomorrow's share");
    assert_eq!(judge.asked().len(), 1);
}
