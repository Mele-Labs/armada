//! The claim, written before the code: **a Job's sketches, drafts and pages read back as one list,
//! newest first, each with its revisions and where its newest stands; a draft proposed under the title
//! of one still open is that draft's next revision, the earlier one withdrawn with nothing sent to the
//! Job; and a sketch he dismissed reads `dismissed`.** `crates/ipc/operations/get_job_artifacts.toml`.
//!
//! And for archiving: **the agent or he archives a stale artifact, and one that waited on him leaves his list
//! with no verdict, the Job told only where he did it; a restore lifts it; and a sketch the Job withdrew
//! reads `withdrawn` where his Dismiss reads `dismissed`.** `crates/ipc/operations/archive_artifact.toml`.
//!
//! And for the For line: **a sketch, a draft and a page each say what they are for in one sentence with the
//! agent's links, Fleet adds the task in progress, its plan step and each open pull request the Job holds,
//! a sentence over 200 characters is refused, and a page's link that is not http(s) is dropped and named.**
//!
//! And for his Edit: **his Edit of a page's or a draft's words is the next revision, his, the page's file holds
//! his bytes, the Job is sent each change, and an Approve says his text; an Edit of anything but words, of a
//! revision not the newest, or of a sketch is refused.** `crates/ipc/operations/save_artifact_edit.toml`.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    AnswerWaiting, ArtifactBy, ArtifactFor, ArtifactForm, ArtifactLink, ArtifactLinkKind, ArtifactRef, ArtifactState, DismissWaiting, DraftHow, ForLink,
    ProposeDraft, SaveArtifactEdit, SessionId, SessionRow, SessionVoice, ShowSketch, WithdrawSketch,
};
use store::{AttachmentState, Holder, KeptAttachment};

use super::session_host::{eventually, rig_placing, Rig};

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;

fn drawn(id: &SessionId, title: &str) -> ShowSketch {
    ipc::decode("a sketch to show", format!(r#"{{"title":"{title}","session_id":"{}","scene":{SCENE}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn proposed(id: &SessionId, title: &str, text: &str) -> ProposeDraft {
    ProposeDraft { title: title.into(), text: text.into(), before: None, pull_request: None, session_id: Some(id.clone()), ..Default::default() }
}

/// A Job with its process up, and where the stand-in holds it.
async fn a_job(rig: &Rig) -> (SessionId, usize) {
    let id = rig.start().await;
    rig.send(&id, "write it up").await;
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id.as_str()) }).await;
    let at = rig.stand_in.starts().iter().position(|one| one.session == id.as_str()).unwrap();
    rig.stand_in.init(at);
    (id, at)
}

/// The Job writes a page for reading under its root, and Fleet keeps it.
async fn page_written(rig: &Rig, id: &SessionId, at: usize) -> String {
    let file = std::path::Path::new(&rig.root).join(".armada/documents/lease.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "<html><head><title>Lease expiry</title></head><body><p>Findings.</p></body></html>").unwrap();
    let path = file.to_string_lossy().into_owned();
    rig.stand_in.says(
        at,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: "toolu_1".into(), detail: CallDetail::of(&format!("{path} +1")) },
            DroneEvent::Answered { call: "toolu_1".into(), failed: false },
        ],
    );
    eventually(|| async {
        rig.fleet.get_job_artifacts(id.clone()).await.unwrap().iter().any(|one| one.form == ArtifactForm::Page)
    })
    .await;
    path
}

fn sent(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_jobs_sketch_page_and_twice_proposed_draft_read_back_as_three_newest_first() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();
    let path = page_written(&rig, &id, at).await;
    let first = rig.fleet.propose_draft(None, proposed(&id, "Description for #41", "First words.")).await.unwrap();
    let first_row = first.waiting_for.iter().find(|one| one.id.starts_with("draft:")).expect("the first draft waits").id.clone();
    let told_before = sent(&rig.rows(&id).await);
    let second = rig.fleet.propose_draft(None, proposed(&id, "Description for #41", "Second words.")).await.unwrap();

    // His list holds one draft, the second, and the first was withdrawn with nothing sent to the Job.
    let drafts: Vec<&str> = second.waiting_for.iter().filter(|one| one.id.starts_with("draft:")).map(|one| one.id.as_str()).collect();
    assert_eq!(drafts.len(), 1, "{:?}", second.waiting_for);
    assert_ne!(drafts[0], first_row);
    let rows = rig.rows(&id).await;
    let first_settled = rows.iter().find_map(|row| match row {
        SessionRow::Draft { id, settled, .. } if format!("draft:{id}") == first_row => Some(settled.clone()),
        _ => None,
    });
    assert_eq!(first_settled.flatten().map(|settled| (settled.how, settled.words)), Some((DraftHow::Withdrawn, None)));
    assert_eq!(sent(&rows), told_before, "nothing is sent for a replaced draft");

    let artifacts = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    let read: Vec<(&str, ArtifactForm, usize, ArtifactState, Option<&str>)> = artifacts
        .iter()
        .map(|one| (one.id.as_str(), one.form, one.revisions.len(), one.state, one.item.as_deref()))
        .collect();
    assert_eq!(
        read,
        [
            ("draft:Description for #41", ArtifactForm::Draft, 2, ArtifactState::Asking, Some(drafts[0])),
            (path.as_str(), ArtifactForm::Page, 1, ArtifactState::Reading, None),
            ("sketch:Request flow", ArtifactForm::Sketch, 1, ArtifactState::Asking, Some("sketch:Request flow")),
        ]
    );
    let draft = &artifacts[0];
    assert_eq!(draft.title, "Description for #41");
    assert_eq!(draft.revisions.iter().map(|one| one.number).collect::<Vec<_>>(), [1, 2]);
    assert!(draft.revisions[0].at < draft.revisions[1].at, "{:?}", draft.revisions);
    assert!(draft.revisions.iter().all(|one| one.by == ArtifactBy::Agent && one.form == ArtifactForm::Draft));
    assert!(artifacts.iter().all(|one| one.words.is_none()));
}

#[tokio::test]
async fn a_sketch_he_dismissed_reads_dismissed() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();
    let dismissal = DismissWaiting { session_id: id.clone(), item_id: "sketch:Request flow".into(), reason: None, misread: false };
    Arc::clone(&rig.fleet).dismiss_waiting(dismissal).await.expect("dismissed");

    let artifacts = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    assert_eq!(artifacts.iter().map(|one| (one.id.as_str(), one.state, one.item.as_deref())).collect::<Vec<_>>(), [("sketch:Request flow", ArtifactState::Dismissed, None)]);
}

fn named(id: &SessionId, artifact: &str) -> ArtifactRef {
    ArtifactRef { artifact: artifact.into(), session_id: Some(id.clone()) }
}

/// The ids his Needs you list holds now.
async fn his_list(rig: &Rig, id: &SessionId) -> Vec<String> {
    rig.fleet.published_hosted(id.as_str()).await.unwrap().waiting_for.into_iter().map(|one| one.id).collect()
}

#[tokio::test]
async fn the_agent_archiving_an_asking_sketch_takes_it_off_his_list_and_sends_nothing() {
    let rig = rig_placing(None);
    let (id, _) = a_job(&rig).await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();
    assert!(his_list(&rig, &id).await.contains(&"sketch:Request flow".to_string()));
    let told_before = sent(&rig.rows(&id).await);

    let archived = Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Agent, named(&id, "sketch:Request flow")).await.expect("archived");

    assert_eq!((archived.state, archived.item.as_deref()), (ArtifactState::Asking, None), "its state as it stood, off his list");
    assert_eq!(archived.archived.map(|one| one.by), Some(ArtifactBy::Agent));
    assert!(!his_list(&rig, &id).await.contains(&"sketch:Request flow".to_string()));
    assert_eq!(sent(&rig.rows(&id).await), told_before, "nothing is sent for the agent's own archive");
    let Err(Refusal::Unacceptable(again)) = Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Agent, named(&id, "sketch:Request flow")).await
    else {
        panic!("one archived already is refused");
    };
    assert_eq!(again.code, "fleet.artifact_archived_already");
}

#[tokio::test]
async fn his_archiving_a_draft_tells_the_job_once_and_a_restore_brings_it_back_to_his_list() {
    let rig = rig_placing(None);
    let (id, _) = a_job(&rig).await;
    let proposed = rig.fleet.propose_draft(None, proposed(&id, "Description for #41", "Words.")).await.unwrap();
    let row = proposed.waiting_for.iter().find(|one| one.id.starts_with("draft:")).expect("the draft waits").id.clone();
    let told_before = sent(&rig.rows(&id).await);

    let archived = Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Owner, named(&id, "draft:Description for #41")).await.expect("archived");

    assert_eq!(archived.archived.as_ref().map(|one| one.by), Some(ArtifactBy::Owner));
    assert!(!his_list(&rig, &id).await.contains(&row));
    let told: Vec<String> = sent(&rig.rows(&id).await).into_iter().skip(told_before.len()).collect();
    assert_eq!(told, ["He archived \"Description for #41\". It is off his list with no verdict."]);

    let restored = rig.fleet.restore_artifact(None, named(&id, "draft:Description for #41")).await.expect("restored");
    assert_eq!((restored.archived, restored.state, restored.item.as_deref()), (None, ArtifactState::Asking, Some(row.as_str())));
    assert!(his_list(&rig, &id).await.contains(&row), "the dismissal the archive wrote is lifted");
    assert_eq!(sent(&rig.rows(&id).await).len(), told_before.len() + 1, "a restore sends nothing");
    let Err(Refusal::Unacceptable(again)) = rig.fleet.restore_artifact(None, named(&id, "draft:Description for #41")).await else {
        panic!("one not archived is refused");
    };
    assert_eq!(again.code, "fleet.artifact_not_archived");
}

#[tokio::test]
async fn archiving_an_id_the_job_does_not_hold_is_refused_naming_the_ids_it_does() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();

    let Err(Refusal::Unacceptable(refused)) = Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Agent, named(&id, "sketch:Storage")).await
    else {
        panic!("an id the job does not hold is refused");
    };
    assert_eq!(refused.code, "fleet.artifact_not_held");
    assert!(refused.message.contains("`sketch:Storage`") && refused.message.contains("sketch:Request flow"), "{}", refused.message);
}

#[tokio::test]
async fn a_sketch_the_job_withdrew_reads_withdrawn_and_one_he_dismissed_reads_dismissed() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();
    rig.fleet.show_sketch(None, drawn(&id, "Storage")).await.unwrap();
    rig.fleet.withdraw_sketch(None, WithdrawSketch { title: "Request flow".into(), session_id: Some(id.clone()) }).await.expect("withdrawn");
    let dismissal = DismissWaiting { session_id: id.clone(), item_id: "sketch:Storage".into(), reason: None, misread: false };
    Arc::clone(&rig.fleet).dismiss_waiting(dismissal).await.expect("dismissed");

    let mut read: Vec<(String, ArtifactState)> =
        rig.fleet.get_job_artifacts(id.clone()).await.unwrap().into_iter().map(|one| (one.id, one.state)).collect();
    read.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(read, [("sketch:Request flow".to_string(), ArtifactState::Withdrawn), ("sketch:Storage".to_string(), ArtifactState::Dismissed)]);
}

fn link(kind: ArtifactLinkKind, label: &str, target: &str) -> ArtifactLink {
    ArtifactLink { kind, label: label.into(), target: target.into() }
}

const FOR: &str = "Picks the store the lease reads; your answer decides which one is built.";

/// The Job holds pull request 41, open, under the title the forge last gave it.
async fn holding_41(rig: &Rig, id: &SessionId) {
    let manifest = rig.fleet.store().lock().await.session(id.as_str()).unwrap().unwrap().manifest_id.expect("a repository");
    let now = rig.fleet.instant().as_str().to_string();
    let detail = [("state".to_string(), "open".to_string()), ("title".to_string(), "Take drafts out".to_string())].into();
    let pull = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "pr".into(),
        manifest_id: manifest,
        target: "41".into(),
        state: AttachmentState::Standing,
        detail,
        since: now.clone(),
        changed_at: now,
    };
    rig.fleet.store().lock().await.attach(&pull, false).unwrap();
}

#[tokio::test]
async fn a_sketch_shown_with_a_for_line_and_a_link_reads_both_then_the_pull_request_the_job_holds() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holding_41(&rig, &id).await;
    let issue = ForLink { label: "Issue 7".into(), url: "https://example.com/issues/7".into() };
    let shown = ShowSketch { for_line: Some(format!(" {FOR} ")), links: vec![issue], ..drawn(&id, "Request flow") };
    rig.fleet.show_sketch(None, shown).await.unwrap();

    let artifacts = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    let wanted = ArtifactFor {
        text: Some(FOR.into()),
        links: vec![
            link(ArtifactLinkKind::Link, "Issue 7", "https://example.com/issues/7"),
            link(ArtifactLinkKind::PullRequest, "#41 Take drafts out", "#41"),
        ],
    };
    assert_eq!(artifacts[0].for_line.as_ref(), Some(&wanted));

    // Drawn again with no For line, the newest revision has only what Fleet knows.
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();
    let artifacts = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    let fleets = ArtifactFor { text: None, links: vec![link(ArtifactLinkKind::PullRequest, "#41 Take drafts out", "#41")] };
    assert_eq!(artifacts[0].for_line, Some(fleets));
}

#[tokio::test]
async fn a_for_line_over_200_characters_or_a_link_not_on_the_web_is_refused_and_a_draft_keeps_its_own_on_its_row() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let long = "a".repeat(201);
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.show_sketch(None, ShowSketch { for_line: Some(long.clone()), ..drawn(&id, "Request flow") }).await
    else {
        panic!("a sketch whose for is too long is refused");
    };
    assert_eq!(refused.code, "fleet.sketch_refused");
    assert!(refused.message.contains("201 characters and holds at most 200"), "{}", refused.message);

    let Err(Refusal::Unacceptable(refused)) =
        rig.fleet.propose_draft(None, ProposeDraft { for_line: Some(long), ..proposed(&id, "Description for #41", "Words.") }).await
    else {
        panic!("a draft whose for is too long is refused");
    };
    assert_eq!(refused.code, "fleet.draft_refused");
    let ftp = ForLink { label: "Spec".into(), url: "ftp://example.com/spec".into() };
    let Err(Refusal::Unacceptable(refused)) =
        rig.fleet.propose_draft(None, ProposeDraft { links: vec![ftp], ..proposed(&id, "Description for #41", "Words.") }).await
    else {
        panic!("a link that is not http(s) is refused");
    };
    assert!(refused.message.contains("links[0] `Spec` is `ftp://example.com/spec`"), "{}", refused.message);
    assert!(rig.fleet.get_job_artifacts(id.clone()).await.unwrap().is_empty(), "nothing kept");

    rig.fleet.propose_draft(None, ProposeDraft { for_line: Some(FOR.into()), ..proposed(&id, "Description for #41", "Words.") }).await.unwrap();
    let said = Some(ArtifactFor { text: Some(FOR.into()), links: Vec::new() });
    let rows = rig.rows(&id).await;
    let on_row = rows.iter().find_map(|row| match row {
        SessionRow::Draft { for_line, .. } => Some(for_line.clone()),
        _ => None,
    });
    assert_eq!(on_row, Some(said.clone()));
    assert_eq!(rig.fleet.get_job_artifacts(id.clone()).await.unwrap()[0].for_line, said);
}

#[tokio::test]
async fn a_page_reads_its_for_meta_and_links_and_a_javascript_link_is_dropped_and_named_to_the_job() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let file = std::path::Path::new(&rig.root).join(".armada/documents/lease.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let head = format!(
        r#"<title>Lease expiry</title><meta name="armada-for" content="{FOR}">
           <link rel="armada-for" href="https://example.com/issues/7" title="Issue 7">
           <link rel="armada-for" href="javascript:alert(1)" title="Run">"#
    );
    std::fs::write(&file, format!("<html><head>{head}</head><body><p>Findings.</p></body></html>")).unwrap();
    let path = file.to_string_lossy().into_owned();
    rig.stand_in.says(
        at,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: "toolu_1".into(), detail: CallDetail::of(&format!("{path} +1")) },
            DroneEvent::Answered { call: "toolu_1".into(), failed: false },
        ],
    );
    eventually(|| async { rig.fleet.get_job_artifacts(id.clone()).await.unwrap().iter().any(|one| one.form == ArtifactForm::Page) }).await;

    let page = rig.fleet.get_job_artifacts(id.clone()).await.unwrap().into_iter().find(|one| one.id == path).unwrap();
    let wanted = ArtifactFor { text: Some(FOR.into()), links: vec![link(ArtifactLinkKind::Link, "Issue 7", "https://example.com/issues/7")] };
    assert_eq!(page.for_line, Some(wanted));
    let told = || async {
        rig.rows(&id).await.into_iter().find_map(|row| match row {
            SessionRow::Message { notice: Some(word), text, .. } if word == "document_sketch" => Some(text),
            _ => None,
        })
    };
    eventually(|| async { told().await.is_some() }).await;
    let told = told().await.unwrap();
    assert!(told.contains("`Run` (javascript:alert(1))") && !told.contains("Issue 7"), "{told}");
}

#[tokio::test]
async fn a_job_working_a_task_of_its_plan_gets_the_task_and_its_plan_step_on_the_for_line() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    super::document_answers::plan_recorded(&rig, &id).await;
    let (group, task) = {
        let mut store = rig.fleet.store().lock().await;
        let mut plan = store.session_plan(id.as_str()).unwrap().expect("a plan");
        plan.groups[0].tasks[0].marked = Some("in_progress".into());
        store.keep_session_plan_progress(id.as_str(), &plan).unwrap();
        (plan.groups[0].id.clone(), plan.groups[0].tasks[0].id.clone())
    };
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();

    let artifacts = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    let wanted = ArtifactFor {
        text: None,
        links: vec![link(ArtifactLinkKind::Task, "Add the table", &task), link(ArtifactLinkKind::PlanStep, "Store", &group)],
    };
    assert_eq!(artifacts[0].for_line, Some(wanted));
}

const LEASE: &str = "<html><head><title>Lease expiry</title></head><body><h1>Lease expiry</h1><h2>Findings</h2><p>Leases expire.</p>\
<table><tr><th>Store</th></tr><tr><td>Redis</td></tr></table></body></html>";

/// The Job writes `html` as its page, and Fleet keeps it as revision 1.
async fn page_holding(rig: &Rig, id: &SessionId, at: usize, html: &str) -> String {
    let file = std::path::Path::new(&rig.root).join(".armada/documents/lease.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, html).unwrap();
    let path = file.to_string_lossy().into_owned();
    rig.stand_in.says(
        at,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: "toolu_1".into(), detail: CallDetail::of(&format!("{path} +2")) },
            DroneEvent::Answered { call: "toolu_1".into(), failed: false },
        ],
    );
    eventually(|| async { rig.fleet.get_job_artifacts(id.clone()).await.unwrap().iter().any(|one| one.id == path) }).await;
    path
}

fn edit(id: &SessionId, artifact: &str, revision: u32, text: &str) -> SaveArtifactEdit {
    SaveArtifactEdit { session_id: id.clone(), artifact: artifact.into(), revision, text: text.into() }
}

#[tokio::test]
async fn his_edit_of_a_heading_and_a_cell_is_his_revision_in_the_pages_file_and_each_change_is_told() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, LEASE).await;
    let told_before = sent(&rig.rows(&id).await);
    let his = LEASE.replace("<h2>Findings</h2>", "<h2>What we found</h2>").replace("<td>Redis</td>", "<td>Postgres</td>");

    let saved = Arc::clone(&rig.fleet).save_artifact_edit(edit(&id, &path, 1, &his)).await.expect("saved");

    assert_eq!(saved.revisions.iter().map(|one| (one.number, one.by)).collect::<Vec<_>>(), [(1, ArtifactBy::Agent), (2, ArtifactBy::Owner)]);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), his, "the page's file holds his bytes");
    let told: Vec<String> = sent(&rig.rows(&id).await).into_iter().skip(told_before.len()).collect();
    let wanted = format!(
        "He edited \"Lease expiry\": revision 2 is his, from revision 1.\n- Top: \"Findings\" → \"What we found\"\n\
         - What we found: \"Redis\" → \"Postgres\"\nThe page at {path} holds his words. Build on them."
    );
    assert_eq!(told, [wanted]);

    // The turn's end reads his bytes again and keeps no third, nor makes his the agent's.
    super::documents::turn_swept(&rig, &id, at, "zz-swept.html").await;
    let page = rig.fleet.get_job_artifacts(id.clone()).await.unwrap().into_iter().find(|one| one.id == path).unwrap();
    assert_eq!(page.revisions.iter().map(|one| one.by).collect::<Vec<_>>(), [ArtifactBy::Agent, ArtifactBy::Owner]);
}

#[tokio::test]
async fn an_edit_that_adds_an_element_is_refused_naming_it_and_keeps_nothing() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, LEASE).await;
    let told_before = sent(&rig.rows(&id).await);
    let his = LEASE.replace("<p>Leases expire.</p>", "<p>Leases expire.</p><p>And renew.</p>");

    let Err(Refusal::Unacceptable(refused)) = Arc::clone(&rig.fleet).save_artifact_edit(edit(&id, &path, 1, &his)).await else {
        panic!("an added element is refused");
    };

    assert_eq!(refused.code, "fleet.edit_not_words");
    assert!(refused.message.contains("`<p>` where revision 1 has `<table>`"), "{}", refused.message);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), LEASE);
    assert_eq!(sent(&rig.rows(&id).await), told_before, "nothing is sent");
    let page = rig.fleet.get_job_artifacts(id.clone()).await.unwrap().into_iter().find(|one| one.id == path).unwrap();
    assert_eq!(page.revisions.len(), 1);
}

#[tokio::test]
async fn an_edit_of_a_revision_not_the_newest_is_refused_naming_the_newest() {
    let rig = rig_placing(None);
    let (id, _) = a_job(&rig).await;
    rig.fleet.propose_draft(None, proposed(&id, "Description for #41", "First words.")).await.unwrap();
    rig.fleet.propose_draft(None, proposed(&id, "Description for #41", "Second words.")).await.unwrap();

    let Err(Refusal::IllegalMove(refused)) = Arc::clone(&rig.fleet).save_artifact_edit(edit(&id, "draft:Description for #41", 1, "His words.")).await
    else {
        panic!("an edit of revision 1 is refused");
    };

    assert_eq!(refused.code, "fleet.edit_not_newest");
    assert!(refused.message.contains("revision 2 is"), "{}", refused.message);
}

#[tokio::test]
async fn his_edit_of_a_draft_becomes_its_rows_text_and_his_approve_says_it() {
    let rig = rig_placing(None);
    let (id, _) = a_job(&rig).await;
    let proposed = rig.fleet.propose_draft(None, proposed(&id, "Description for #41", "Takes drafts out.\nKeeps the lease.")).await.unwrap();
    let item = proposed.waiting_for.iter().find(|one| one.id.starts_with("draft:")).expect("the draft waits").id.clone();
    let told_before = sent(&rig.rows(&id).await);
    let his = "Takes drafts out.\nKeeps the lease for a week.";

    let saved = Arc::clone(&rig.fleet).save_artifact_edit(edit(&id, "draft:Description for #41", 1, his)).await.expect("saved");

    assert_eq!(saved.revisions.iter().map(|one| one.by).collect::<Vec<_>>(), [ArtifactBy::Agent, ArtifactBy::Owner]);
    assert_eq!((saved.state, saved.item.as_deref()), (ArtifactState::Asking, Some(item.as_str())), "an Edit settles nothing");
    let text = rig.rows(&id).await.into_iter().find_map(|row| match row {
        SessionRow::Draft { id: row, text, .. } if format!("draft:{row}") == item => Some(text),
        _ => None,
    });
    assert_eq!(text.as_deref(), Some(his));
    let told: Vec<String> = sent(&rig.rows(&id).await).into_iter().skip(told_before.len()).collect();
    assert_eq!(
        told,
        ["He edited \"Description for #41\": revision 2 is his, from revision 1.\n- Keeps the lease.\n+ Keeps the lease for a week.\n\
          Use his text as the draft from now on."]
    );

    let approve = AnswerWaiting { session_id: id.clone(), item_id: item, choice: Some(0), text: None, mode: None, fixed: None };
    Arc::clone(&rig.fleet).answer_waiting(approve).await.unwrap();
    let approved = sent(&rig.rows(&id).await).pop().unwrap();
    assert!(approved.ends_with(&format!("\n\n{his}")), "{approved}");
}

#[tokio::test]
async fn an_edit_of_a_sketch_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();

    let Err(Refusal::Unacceptable(refused)) = Arc::clone(&rig.fleet).save_artifact_edit(edit(&id, "sketch:Request flow", 1, "Words.")).await else {
        panic!("a sketch takes no edit");
    };

    assert_eq!((refused.code.as_str(), refused.message.as_str()), ("fleet.edit_not_words", "a sketch takes notes, not edits"));
}
