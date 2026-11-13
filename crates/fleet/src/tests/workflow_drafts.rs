//! A workflow's draft and a piece's: kept by Fleet while it is edited, written by Publish through the save, and where
//! an agent's drafted change lands. The model is `testkit::FakeJudge`, as `workflow_changes` has it, and what a save
//! writes is the composition root's and is `Planted` here, which keeps what it was asked to save.

use std::sync::Arc;

use api::Lifecycles;
use axum::http::StatusCode;
use ipc::{
    AutonomyToSave, DraftBy, LifecyclePiece, LifecyclePieceKind, PieceDraft, PieceScope, PublishWorkflowDraft, PutPieceDraft, PutWorkflowDraft,
    SaveLifecyclePiece, WorkflowChange, WorkflowDefinition, WorkflowDraft, WorkflowId, WorkflowPublished, WorkflowScope,
};
use testkit::FakeWorkProduct;

use crate::repositories::{SavedWorkflow, WorkflowNotSaved};
use crate::tests::daemon::a_fleet;
use crate::tests::http::call;
use crate::tests::repositories::code;
use crate::tests::tmp::TempDir;
use crate::tests::workflow_changes::{decoded, rig, with_refine, Rig, MANIFEST};
use crate::tests::workflow_page::{at, carried_id, read, STAGED};

fn put_body(source: &str, project: Option<&str>, text: &str) -> String {
    ipc::encode(&PutWorkflowDraft {
        workflow_id: WorkflowId::carried("staged"),
        source: source.into(),
        project: project.map(str::to_string),
        text: text.into(),
        summary: None,
    })
    .expect("plain data")
}

fn publish_body(workflow: &str, source: &str, scope: Option<WorkflowScope>) -> String {
    ipc::encode(&PublishWorkflowDraft { workflow_id: WorkflowId::carried(workflow), source: source.into(), project: None, scope }).expect("plain data")
}

fn saved(workflow: &str, file: &str) -> SavedWorkflow {
    SavedWorkflow { id: carried_id(workflow), file: file.to_string(), replaced: true }
}

impl Rig {
    async fn put(&self, source: &str, text: &str) -> (StatusCode, Vec<u8>) {
        call(&self.app, "PUT", "/workflows/draft", &put_body(source, None, text)).await
    }

    async fn draft(&self, source: &str) -> (StatusCode, Vec<u8>) {
        call(&self.app, "GET", &format!("/workflows/draft?workflow_id=staged&source={source}"), "").await
    }
}

/// **A put begins the draft from what is published, and a get reads it back** with one change, by `you`, holding the text whole.
#[tokio::test]
async fn a_put_then_a_get_returns_the_text_with_one_change_by_you_and_the_published_text_as_its_base() {
    let rig = rig(&[]).await;
    let after = with_refine();

    let (status, body) = rig.put("repository", &after).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (status, body) = rig.draft("repository").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!((draft.text.as_str(), draft.base.as_str(), draft.unfit.as_deref()), (after.as_str(), STAGED, None));
    assert_eq!((draft.workflow_id.as_str(), draft.source.as_str(), draft.project), ("staged", "repository", None));
    assert_eq!(draft.changes.len(), 1);
    assert_eq!((draft.changes[0].by, draft.changes[0].text.as_str(), draft.changes[0].change), (DraftBy::You, after.as_str(), None));
    assert!(rig.planted.saves().is_empty(), "a draft writes nothing");

    let (status, body) = rig.draft("kit").await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.workflow_draft_none"), "a draft is one place's");
}

/// **Each put adds a change, and putting an earlier text again is how the editor steps back**: the list keeps every one in
/// order, and a put of what the draft already holds adds none.
#[tokio::test]
async fn each_put_adds_a_change_in_order_and_one_that_repeats_the_draft_adds_none() {
    let rig = rig(&[]).await;
    let (one, two) = (with_refine(), format!("{}\n# a second thought", with_refine()));
    for text in [&one, &two, &one, &one] {
        let (status, body) = rig.put("repository", text).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    }
    let (_, body) = rig.draft("repository").await;
    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!(draft.changes.iter().map(|change| change.text.as_str()).collect::<Vec<_>>(), [one.as_str(), two.as_str(), one.as_str()]);
    assert_eq!(draft.text, one);
    assert_eq!(draft.base, STAGED, "it keeps what it began from");

    let (status, _) = call(&rig.app, "DELETE", "/workflows/draft?workflow_id=staged&source=repository", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rig.draft("repository").await.0, StatusCode::NOT_FOUND, "discarded with its changes");
    let (status, body) = call(&rig.app, "DELETE", "/workflows/draft?workflow_id=staged&source=repository", "").await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.workflow_draft_none"));
}

/// **Publish writes through the save with overwrite, in the draft's own place**, deletes the draft, and the repository reads the
/// folders again so it runs what was written.
#[tokio::test]
async fn publish_saves_the_draft_in_its_place_with_overwrite_deletes_it_and_the_repository_reads_again() {
    let rig = rig(&[]).await;
    let after = with_refine();
    rig.put("repository", &after).await;
    rig.planted.will_save(Ok(saved("staged", "/repo/.armada/workflows/staged.json")));
    let reads = rig.planted.read_for().len();

    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish_body("staged", "repository", None)).await;

    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let published: WorkflowPublished = decoded("a publish", &body);
    assert_eq!((published.saved.file.as_str(), published.saved.runs_from.as_deref()), ("/repo/.armada/workflows/staged.json", Some("repository")));
    let saves = rig.planted.saves();
    assert_eq!(saves.len(), 1);
    assert_eq!((saves[0].scope.clone(), saves[0].definition.as_str(), saves[0].overwrite), (WorkflowScope::Repository, after.as_str(), true));
    assert!(rig.planted.read_for().len() > reads, "the folders were read again, so the repository runs it");
    assert_eq!(rig.draft("repository").await.0, StatusCode::NOT_FOUND, "the draft is gone");

    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish_body("staged", "repository", None)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.workflow_draft_none"));
}

/// **A draft of one of Armada's has no scope until it is published**, and then the owner picks any of the three; for any
/// other source a scope that names another place is refused.
#[tokio::test]
async fn publishing_armadas_draft_needs_a_scope_and_writes_that_one_and_another_draft_may_not_name_another_place() {
    let rig = rig(&[]).await;
    let (status, body) = call(&rig.app, "GET", "/workflows/definition?workflow_id=feature&source=armada", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let carried = decoded::<WorkflowDefinition>("a definition", &body).definition;
    let edited = format!("{carried}\n");
    let put = ipc::encode(&PutWorkflowDraft { workflow_id: WorkflowId::carried("feature"), source: "armada".into(), text: edited.clone(), ..Default::default() }).unwrap();
    let (status, body) = call(&rig.app, "PUT", "/workflows/draft", &put).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(decoded::<WorkflowDraft>("a draft", &body).base, carried, "for Armada's the base is the carried text");

    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish_body("feature", "armada", None)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_draft_scope_needed"));
    assert!(rig.planted.saves().is_empty(), "nothing was written without a scope");

    rig.planted.will_save(Ok(saved("feature", "/kit/workflows/feature.json")));
    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish_body("feature", "armada", Some(WorkflowScope::Kit))).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let saves = rig.planted.saves();
    assert_eq!((saves.len(), saves[0].scope.clone(), saves[0].overwrite), (1, WorkflowScope::Kit, true), "Kit's copy is written");
    assert_eq!(saves[0].definition, edited);

    rig.put("repository", &with_refine()).await;
    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish_body("staged", "repository", Some(WorkflowScope::Kit))).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_draft_scope_differs"));
    assert_eq!(rig.planted.saves().len(), 1, "nothing more was written");
}

/// **A file edited by hand since the draft began makes the publish stale**: nothing is written and the draft stays.
#[tokio::test]
async fn after_a_hand_edit_of_the_file_publish_is_refused_as_stale_and_the_draft_is_kept() {
    let rig = rig(&[]).await;
    rig.put("repository", &with_refine()).await;
    let edited = format!("{STAGED}# edited by hand\n");
    rig.planted.will_read(read(vec![config::Written::in_repository(at("repo", "staged.json"), edited)]));
    rig.fleet.workflows_changed(&rig.home.path().to_string_lossy());

    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish_body("staged", "repository", None)).await;

    assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.workflow_draft_stale"));
    assert!(rig.planted.saves().is_empty(), "nothing was written");
    assert_eq!(rig.draft("repository").await.0, StatusCode::OK, "the draft is kept to be read against what is there now");
}

/// **A draft that would not load says so, and is not published**: `unfit` carries the loader's sentence, and the
/// publish is refused with that sentence and no save asked for.
#[tokio::test]
async fn an_unfit_draft_carries_the_loaders_sentence_and_its_publish_is_refused_with_nothing_written() {
    let rig = rig(&[]).await;
    let unfit = || Err(WorkflowNotSaved::Unfit { why: "step `refine` needs `lint`, and the declared Checks are `test`".into() });
    rig.planted.will_check(vec![unfit()]);
    let (status, body) = rig.put("repository", "half of a definition").await;
    assert_eq!(status, StatusCode::OK, "a draft may be half-edited: {}", String::from_utf8_lossy(&body));
    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!(draft.unfit.as_deref(), Some("step `refine` needs `lint`, and the declared Checks are `test`"));

    rig.planted.will_check(vec![unfit()]);
    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish_body("staged", "repository", None)).await;

    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_unfit"));
    assert!(rig.planted.saves().is_empty(), "nothing was written");
    assert_eq!(rig.draft("repository").await.0, StatusCode::OK, "the draft is kept");
}

/// **A draft whose text declares another id is unfit**, since publishing it would write a second workflow.
#[tokio::test]
async fn a_draft_that_declares_another_workflow_id_is_unfit_and_is_not_published() {
    let rig = rig(&[]).await;
    let renamed = STAGED.replace("workflow_id: staged", "workflow_id: other");
    let (_, body) = rig.put("repository", &renamed).await;
    assert!(decoded::<WorkflowDraft>("a draft", &body).unfit.is_some_and(|why| why.contains("`other`") && why.contains("`staged`")));
    let (status, body) = call(&rig.app, "POST", "/workflows/draft/publish", &publish_body("staged", "repository", None)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_unfit"));
    assert!(rig.planted.saves().is_empty());
}

/// **A project's draft is named by the project**, and one for a project Fleet holds none of is refused.
#[tokio::test]
async fn a_projects_draft_names_its_project_and_an_unnamed_or_unknown_one_is_refused() {
    let rig = rig(&[]).await;
    let (status, body) = rig.put("project", &with_refine()).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_project_unnamed"));
    let (status, body) = call(&rig.app, "PUT", "/workflows/draft", &put_body("project", Some("nowhere"), &with_refine())).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.workflow_project_unknown"));
}

// ---- an agent's edit lands in the draft ----------------------------------------------------------------------

/// **A change he asks for lands in his draft as the agent's**, marked `drafted`, with its words as the summary and its id as
/// `change`; the draft began from what was published.
#[tokio::test]
async fn an_ask_whose_drafting_finishes_lands_in_the_draft_as_an_agent_change_marked_drafted() {
    let after = with_refine();
    let rig = rig(&[&after]).await;

    let ready = rig.asked("add a Refine step before Deliver").await;

    assert!(ready.drafted, "{ready:?}");
    let (status, body) = rig.draft("repository").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!((draft.text.as_str(), draft.base.as_str()), (after.as_str(), STAGED));
    assert_eq!(draft.changes.len(), 1);
    assert_eq!(
        (draft.changes[0].by, draft.changes[0].summary.as_deref(), draft.changes[0].change),
        (DraftBy::Agent, Some("add a Refine step before Deliver"), Some(ready.id))
    );
    assert!(rig.planted.saves().is_empty(), "nothing is written; the owner publishes");
}

/// **Where a draft exists the agent rewrites it**, so `before` is the draft's text, and its change goes on the end.
#[tokio::test]
async fn an_ask_rewrites_the_draft_where_there_is_one_and_adds_its_change_after_his() {
    let hand = format!("{STAGED}# by hand\n");
    let after = hand.replace("name: staged", "name: staged two");
    let rig = rig(&[&after]).await;
    rig.put("repository", &hand).await;

    let ready = rig.asked("rename it").await;

    assert_eq!((ready.before.as_str(), ready.drafted), (hand.as_str(), true), "{ready:?}");
    let (_, body) = rig.draft("repository").await;
    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!(draft.changes.iter().map(|change| change.by).collect::<Vec<_>>(), [DraftBy::You, DraftBy::Agent]);
    assert_eq!(draft.text.trim(), after.trim());
}

/// **A draft that moved while the agent wrote leaves the change ready and unlanded**, for Approve as before.
#[tokio::test]
async fn a_draft_that_is_no_longer_what_the_change_was_made_from_leaves_it_ready_and_not_drafted() {
    let rig = rig(&[&with_refine()]).await;
    let begun = {
        let mut kept = rig.fleet.store().lock().await;
        let new = store::NewWorkflowChange {
            manifest_id: MANIFEST,
            workflow_id: "staged",
            source: "repository",
            project: "",
            step_id: None,
            asked: Some("add Refine"),
            before_text: STAGED,
        };
        let id = kept.begin_workflow_change(&new, "2026-11-13T09:00:00.000Z").unwrap();
        let key = store::DraftKey { kind: "workflow".into(), source: "repository".into(), manifest_id: MANIFEST.into(), item_id: "staged".into(), ..Default::default() };
        let you = store::NewDraftChange { made_by: store::DRAFT_BY_YOU, text: "# he moved on", summary: None, change: None };
        kept.put_draft_change(&key, STAGED, &you, "2026-11-13T09:01:00.000Z").unwrap();
        id
    };

    Arc::clone(&rig.fleet).change_drafted(begun).await;

    let ready = rig.settled(begun).await;
    assert_eq!((ready.state, ready.drafted), (ipc::ChangeState::Ready, false), "{ready:?}");
    let (_, body) = rig.draft("repository").await;
    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!((draft.text.as_str(), draft.changes.len()), ("# he moved on", 1), "his draft is as he left it");
}

/// **An ask on one of Armada's own is accepted**, and its result lands in a draft that has no scope, begun from the carried text.
#[tokio::test]
async fn an_ask_on_an_armada_workflow_is_accepted_and_its_result_lands_in_a_scopeless_draft() {
    let carried = config::carried().into_iter().find(|one| one.path().ends_with("feature.json")).expect("Armada carries feature").text().to_string();
    let after = carried.replace("\"name\": \"feature\"", "\"name\": \"feature two\"");
    assert_ne!(after, carried);
    let rig = rig(&[&after]).await;
    let asked = ipc::encode(&ipc::AskWorkflowChange {
        workflow_id: WorkflowId::carried("feature"),
        source: "armada".into(),
        text: "call it feature two".into(),
        ..Default::default()
    })
    .unwrap();

    let (status, body) = rig.post("/workflows/changes/ask", &asked).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let begun: WorkflowChange = decoded("a change", &body);
    let ready = rig.settled_in("feature", "armada", begun.id).await;

    assert!(ready.drafted, "{ready:?}");
    let (status, body) = call(&rig.app, "GET", "/workflows/draft?workflow_id=feature&source=armada", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let draft: WorkflowDraft = decoded("a draft", &body);
    assert_eq!((draft.text.as_str(), draft.base.as_str()), (after.trim(), carried.as_str()));
    assert_eq!((draft.changes[0].by, draft.changes[0].change), (DraftBy::Agent, Some(begun.id)));
    assert!(rig.planted.saves().is_empty(), "the copy is made on Publish, at the scope he picks");
    let (status, body) = rig.post("/workflows/changes/approve", &ipc::encode(&ipc::WorkflowChangeRef { change: begun.id }).unwrap()).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "fleet.workflow_is_armadas"), "Approve has no file to write");
}

impl Rig {
    /// The change once it is no longer drafting, read from `workflow` in `source`.
    async fn settled_in(&self, workflow: &str, source: &str, id: i64) -> WorkflowChange {
        for _ in 0..250 {
            let (_, body) = call(&self.app, "GET", &format!("/workflows/changes?workflow_id={workflow}&source={source}"), "").await;
            let found = decoded::<ipc::WorkflowChanges>("changes", &body).changes.into_iter().find(|one| one.id == id);
            if let Some(one) = found.filter(|one| one.state != ipc::ChangeState::Drafting) {
                return one;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("change {id} was still drafting after five seconds");
    }
}

// ---- a piece's draft -------------------------------------------------------------------------------------------

fn a_layer(defaults: &[(&str, &str)]) -> String {
    let defaults = defaults.iter().map(|(kind, autonomy)| (kind.to_string(), autonomy.to_string())).collect();
    ipc::encode(&LifecyclePiece::Autonomy { autonomy: AutonomyToSave { id: "mine".into(), scope: PieceScope::Everywhere, defaults } }).unwrap()
}

fn layer_key() -> ipc::DeleteLifecyclePiece {
    ipc::DeleteLifecyclePiece { kind: LifecyclePieceKind::Autonomy, id: "mine".into(), scope: PieceScope::Everywhere }
}

fn put_layer(text: String) -> PutPieceDraft {
    PutPieceDraft { kind: LifecyclePieceKind::Autonomy, id: "mine".into(), scope: PieceScope::Everywhere, text, summary: None }
}

/// **A piece's draft publishes through the piece save**, writing Kit's file for it; the draft's base is empty where Kit held none.
#[tokio::test]
async fn a_piece_draft_publishes_through_the_piece_save_and_is_gone_after() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let text = a_layer(&[("tracker_write", "acts")]);

    let put = fleet.put_piece_draft(put_layer(text.clone())).await.expect("kept");
    assert_eq!((put.text.as_str(), put.base.as_str(), put.unfit.as_deref(), put.changes.len()), (text.as_str(), "", None, 1));
    assert_eq!(put.changes[0].by, DraftBy::You);

    let published = fleet.publish_piece_draft(ipc::PublishPieceDraft { kind: LifecyclePieceKind::Autonomy, id: "mine".into(), scope: PieceScope::Everywhere }).await.expect("published");

    assert!(!published.saved.replaced);
    let file = home.path().join("kit").join("autonomy").join("mine.yml");
    assert!(std::fs::read_to_string(&file).expect("Kit's file").contains("tracker_write"), "{}", file.display());
    let gone = fleet.get_piece_draft(layer_key()).await.expect_err("discarded by the publish");
    assert_eq!(gone.error().code, "fleet.piece_draft_none");
}

/// **A piece draft that Kit's parser would refuse says so, and a piece Kit changed since is stale.** Nothing is written for either.
#[tokio::test]
async fn an_unfit_piece_draft_is_refused_with_nothing_written_and_one_kit_changed_since_is_stale() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let kit = home.path().join("kit").join("autonomy").join("mine.yml");
    let publish = || ipc::PublishPieceDraft { kind: LifecyclePieceKind::Autonomy, id: "mine".into(), scope: PieceScope::Everywhere };

    let put = fleet.put_piece_draft(put_layer(a_layer(&[("no_such_kind_of_act", "acts")]))).await.expect("a half-edited draft is kept");
    assert!(put.unfit.is_some(), "{put:?}");
    let refused = fleet.publish_piece_draft(publish()).await.expect_err("unfit");
    assert_eq!(refused.error().code, "fleet.piece_unfit");
    assert!(!kit.exists(), "nothing was written");

    let other_id = fleet.put_piece_draft(PutPieceDraft { text: a_layer(&[("tracker_write", "acts")]).replace("mine", "another"), ..put_layer(String::new()) }).await.expect("kept");
    assert!(other_id.unfit.is_some_and(|why| why.contains("another") && why.contains("mine")), "a draft of one piece cannot be another's");

    fleet.put_piece_draft(put_layer(a_layer(&[("tracker_write", "acts")]))).await.expect("kept");
    let piece: LifecyclePiece = ipc::decode("a piece", a_layer(&[("forge_write", "asks")]).as_bytes()).unwrap();
    fleet.save_lifecycle_piece(SaveLifecyclePiece { piece, overwrite: false }).await.expect("Kit's own save, since the draft began");
    let before = std::fs::read_to_string(&kit).expect("written");
    let stale = fleet.publish_piece_draft(publish()).await.expect_err("stale");
    assert_eq!(stale.error().code, "fleet.piece_draft_stale");
    assert_eq!(std::fs::read_to_string(&kit).unwrap(), before, "Kit's file is as it was");
    let _: PieceDraft = fleet.get_piece_draft(layer_key()).await.expect("the draft is kept");
}
