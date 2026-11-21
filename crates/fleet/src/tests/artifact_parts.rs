//! The claim, written before the code: **one artifact grows, with no fixed form. A Job adds the parts it needs, a
//! drawing, words, takes with a vote and buttons, and he answers each part that asks on its own; the Job is told each
//! answer naming the artifact and the part; the artifact settles once every ask is answered; and Needs you holds one
//! item for it, naming what is still open.** And **one artifact is open per Job**: a sketch, then a draft, then a page
//! that asks, shown while the first is open, are one artifact of three parts and one item. A retitled drawing is the
//! earlier one's next revision, and an artifact the Job archives while it asks has its asks withdrawn.
//! `crates/ipc/operations/add_part.toml`, `answer_part.toml`, `docs/concepts/artifact.md`, *Parts*.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use ipc::{AddPart, AnswerPart, ArtifactBy, ArtifactForm, ArtifactRef, ArtifactState, PartHow, PartKind, PartState, SessionId, ShowSketch};

use super::artifacts::a_job;
use super::session_host::{eventually, rig_placing, Rig};

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;
const TAKE_A: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Left"}],"edges":[]}"#;
const TAKE_B: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Right"}],"edges":[]}"#;

fn drawn(id: &SessionId, title: &str, against: Option<&str>) -> ShowSketch {
    let against = against.map(|one| format!(r#","against":"{one}""#)).unwrap_or_default();
    ipc::decode("a sketch", format!(r#"{{"title":"{title}","session_id":"{}","scene":{SCENE}{against}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn added(id: &SessionId, body: &str) -> AddPart {
    ipc::decode("a part", format!(r#"{{"session_id":"{}",{body}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn words(id: &SessionId, title: &str, part: &str, text: &str) -> AddPart {
    added(id, &format!(r#""title":"{title}","part":{{"id":"{part}","kind":"words","text":"{text}"}}"#))
}

fn takes(id: &SessionId, title: &str) -> AddPart {
    added(
        id,
        &format!(
            r#""title":"{title}","part":{{"id":"layout","kind":"takes","question":"Which layout?","takes":[{{"id":"a","title":"Left","scene":{TAKE_A}}},{{"id":"b","title":"Right","scene":{TAKE_B}}}]}}"#
        ),
    )
}

fn buttons(id: &SessionId, title: &str) -> AddPart {
    added(id, &format!(r#""title":"{title}","part":{{"id":"go","kind":"buttons","actions":[{{"id":"ship","act":"message","label":"Ship it","brief":"ship it"}}]}}"#))
}

fn answer(id: &SessionId, artifact: &str, part: &str, drawn: &str, how: PartHow) -> AnswerPart {
    AnswerPart {
        session_id: id.clone(),
        artifact: artifact.into(),
        part: part.into(),
        drawn: drawn.into(),
        how,
        take: None,
        words: (how == PartHow::SentBack).then(|| String::from("Change it.")),
        notes: Default::default(),
        extras: Default::default(),
    }
}

fn named(id: &SessionId, artifact: &str) -> ArtifactRef {
    ArtifactRef { artifact: artifact.into(), session_id: Some(id.clone()) }
}

fn told(rows: &[ipc::SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            ipc::SessionRow::Message { from: ipc::SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The items his Needs you list holds now, as `(id, text)`.
async fn his_list(rig: &Rig, id: &SessionId) -> Vec<(String, String)> {
    rig.fleet.published_hosted(id.as_str()).await.unwrap().waiting_for.into_iter().map(|one| (one.id, one.text)).collect()
}

async fn artifacts(rig: &Rig, id: &SessionId) -> Vec<ipc::JobArtifact> {
    rig.fleet.get_job_artifacts(id.clone()).await.unwrap()
}

/// A page that asks, written under the Job's root as the Job writes one, and kept.
async fn asking_page(rig: &Rig, id: &SessionId, at: usize) -> String {
    let file = std::path::Path::new(&rig.root).join(".armada/documents/lease.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(
        &file,
        r#"<html><head><title>Lease expiry</title><meta name="armada-review" content="approve"></head><body><p>Findings.</p></body></html>"#,
    )
    .unwrap();
    let path = file.to_string_lossy().into_owned();
    rig.stand_in.says(
        at,
        vec![
            DroneEvent::Called { tool: "Write".into(), call: "toolu_1".into(), detail: CallDetail::of(&format!("{path} +1")) },
            DroneEvent::Answered { call: "toolu_1".into(), failed: false },
        ],
    );
    eventually(|| async { artifacts(rig, id).await.iter().any(|one| one.parts.iter().any(|part| part.content.kind() == PartKind::Page)) }).await;
    path
}

#[tokio::test]
async fn one_artifact_grows_a_drawing_takes_and_words_and_each_ask_is_answered_alone() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Cache design", None)).await.unwrap();
    rig.fleet.add_part(None, words(&id, "Cache design", "notes", "Evict by age.")).await.unwrap();
    rig.fleet.add_part(None, takes(&id, "Cache design")).await.unwrap();
    rig.fleet.add_part(None, buttons(&id, "Cache design")).await.unwrap();

    // One artifact, four parts. The buttons ask nothing, and nor does the drawing once takes joined it.
    let held = artifacts(&rig, &id).await;
    assert_eq!(held.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["sketch:Cache design"]);
    let artifact = &held[0];
    let parts: Vec<(&str, PartKind, bool)> = artifact.parts.iter().map(|part| (part.id.as_str(), part.content.kind(), part.asks)).collect();
    assert_eq!(
        parts,
        [
            ("sketch:Cache design", PartKind::Drawing, false),
            ("notes", PartKind::Words, true),
            ("layout", PartKind::Takes, true),
            ("go", PartKind::Buttons, false),
        ]
    );
    assert_eq!((artifact.form, artifact.state), (ArtifactForm::Sketch, ArtifactState::Asking), "form reads as the first part's kind");

    // One Needs you item, naming what is open, never counting it.
    let list = his_list(&rig, &id).await;
    assert_eq!(list, [("sketch:Cache design".to_string(), "Cache design: notes, layout".to_string())]);

    // Words: Approve alone. The Job is told naming the artifact and the part, and the item names what is left.
    let notes = artifact.parts.iter().find(|part| part.id == "notes").unwrap();
    let after = Arc::clone(&rig.fleet).answer_part(answer(&id, "sketch:Cache design", "notes", &notes.drawn, PartHow::Approved)).await.unwrap();
    assert_eq!(after.state, ArtifactState::Asking, "two asks are still open");
    assert_eq!(told(&rig.rows(&id).await).pop().unwrap(), "Approved part \"notes\" of \"Cache design\".\nGo on from it as written.");
    assert_eq!(his_list(&rig, &id).await, [("sketch:Cache design".to_string(), "Cache design: layout".to_string())]);

    // The drawing informs: it has nothing to answer.
    let drawing = after.parts.iter().find(|part| part.content.kind() == PartKind::Drawing).unwrap();
    let Err(Refusal::Unacceptable(informs)) =
        Arc::clone(&rig.fleet).answer_part(answer(&id, "sketch:Cache design", "sketch:Cache design", &drawing.drawn, PartHow::SentBack)).await
    else {
        panic!("a drawing beside takes asks nothing")
    };
    assert_eq!(informs.code, "fleet.part_not_open");

    // A takes part is answered with a pick, and a plain Approve of the whole is refused while one waits.
    let layout = artifacts(&rig, &id).await[0].parts.iter().find(|part| part.id == "layout").cloned().unwrap();
    let mut approve = answer(&id, "sketch:Cache design", "layout", &layout.drawn, PartHow::Approved);
    let Err(Refusal::Unacceptable(wrong)) = Arc::clone(&rig.fleet).answer_part(approve.clone()).await else { panic!("a takes part is not approved") };
    assert_eq!(wrong.code, "fleet.part_how");
    assert!(wrong.message.contains("send a pick"), "it says what to send: {}", wrong.message);
    let item = ipc::ItemAnswer { session_id: id.clone(), item: "sketch:Cache design".into(), how: ipc::ItemHow::Approve, notes: Vec::new(), extras: Default::default() };
    let whole = Arc::clone(&rig.fleet).answer_items(ipc::AnswerItems { items: vec![item] }, Vec::new()).await;
    let ipc::ItemSettled::Refused { message, .. } = &whole.outcomes[0].outcome else { panic!("an Approve of the whole is refused while a pick is open") };
    assert!(message.contains("send a pick"), "{message}");
    approve.how = PartHow::Picked;
    approve.take = Some("b".into());
    approve.drawn = layout.revision.to_string();
    let done = Arc::clone(&rig.fleet).answer_part(AnswerPart { drawn: layout.drawn.clone(), ..approve }).await.unwrap();

    // Every ask answered: the artifact settles approved and leaves his list.
    assert_eq!((done.state, done.item.as_deref()), (ArtifactState::Approved, None));
    let states: Vec<(&str, PartState)> = done.parts.iter().map(|part| (part.id.as_str(), part.state)).collect();
    assert_eq!(
        states,
        [("sketch:Cache design", PartState::Open), ("notes", PartState::Approved), ("layout", PartState::Picked), ("go", PartState::Open)]
    );
    assert!(his_list(&rig, &id).await.is_empty());
    let said = told(&rig.rows(&id).await);
    assert!(said.iter().any(|one| one.contains("layout") && one.contains("Right")), "the pick is told naming the part and the take: {said:?}");

    // A part answered is not answered twice, and a stale revision is a 409.
    let Err(Refusal::Unacceptable(again)) =
        Arc::clone(&rig.fleet).answer_part(answer(&id, "sketch:Cache design", "notes", &notes.drawn, PartHow::Approved)).await
    else {
        panic!("answered already");
    };
    assert_eq!(again.code, "fleet.part_not_open");
}

/// His pick of take `b` on the takes part `layout` of the artifact at `root`, as he reads it now.
async fn picked(rig: &Rig, id: &SessionId, root: &str) -> Result<ipc::JobArtifact, Refusal> {
    let layout = artifacts(rig, id).await.into_iter().find(|one| one.id == root).unwrap().parts.into_iter().find(|part| part.id == "layout").unwrap();
    let mut pick = answer(id, root, "layout", &layout.drawn, PartHow::Picked);
    pick.take = Some("b".into());
    Arc::clone(&rig.fleet).answer_part(pick).await
}

#[tokio::test]
async fn a_sketch_with_takes_leaves_needs_you_on_one_pick_whichever_was_shown_first() {
    // The sketch first, then the takes: the drawing stops asking once they join.
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Catalog", None)).await.unwrap();
    assert_eq!(his_list(&rig, &id).await, [("sketch:Catalog".to_string(), "Catalog".to_string())]);
    rig.fleet.add_part(None, takes(&id, "Catalog")).await.unwrap();
    assert_eq!(his_list(&rig, &id).await, [("sketch:Catalog".to_string(), "Catalog: layout".to_string())], "the takes alone ask");
    let done = picked(&rig, &id, "sketch:Catalog").await.unwrap();
    assert_eq!((done.state, done.item.as_deref()), (ArtifactState::Approved, None));
    assert!(his_list(&rig, &id).await.is_empty(), "one pick settles it");

    // The takes first, then a sketch: it joins and asks nothing.
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.add_part(None, takes(&id, "Catalog")).await.unwrap();
    rig.fleet.show_sketch(None, drawn(&id, "Catalog context", None)).await.unwrap();
    let held = artifacts(&rig, &id).await;
    assert_eq!(held.len(), 1, "one artifact: {:?}", held.iter().map(|one| &one.id).collect::<Vec<_>>());
    let drawing = held[0].parts.iter().find(|part| part.content.kind() == PartKind::Drawing).expect("the drawing joined");
    assert!(!drawing.asks, "a drawing beside takes informs");
    let root = held[0].id.clone();
    assert_eq!(his_list(&rig, &id).await.len(), 1);
    picked(&rig, &id, &root).await.unwrap();
    assert!(his_list(&rig, &id).await.is_empty(), "one pick settles it");
}

#[tokio::test]
async fn words_that_ask_beside_a_sketch_and_takes_still_wait_after_the_pick_and_approve_all_is_refused_for_the_takes() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Catalog", None)).await.unwrap();
    rig.fleet.add_part(None, takes(&id, "Catalog")).await.unwrap();
    rig.fleet.add_part(None, words(&id, "Catalog", "notes", "Group by area.")).await.unwrap();

    // Approve of the whole never stands in for the pick.
    let item = ipc::ItemAnswer { session_id: id.clone(), item: "sketch:Catalog".into(), how: ipc::ItemHow::Approve, notes: Vec::new(), extras: Default::default() };
    let whole = Arc::clone(&rig.fleet).answer_items(ipc::AnswerItems { items: vec![item] }, Vec::new()).await;
    let ipc::ItemSettled::Refused { message, .. } = &whole.outcomes[0].outcome else { panic!("an Approve of the whole is refused while a pick is open") };
    assert!(message.contains("send a pick"), "{message}");

    // The pick answers the takes; the words still ask.
    let after = picked(&rig, &id, "sketch:Catalog").await.unwrap();
    assert_eq!(after.state, ArtifactState::Asking);
    assert_eq!(his_list(&rig, &id).await, [("sketch:Catalog".to_string(), "Catalog: notes".to_string())]);
    let notes = after.parts.iter().find(|part| part.id == "notes").unwrap();
    let done = Arc::clone(&rig.fleet).answer_part(answer(&id, "sketch:Catalog", "notes", &notes.drawn, PartHow::Approved)).await.unwrap();
    assert_eq!(done.state, ArtifactState::Approved);
    assert!(his_list(&rig, &id).await.is_empty());
}

#[tokio::test]
async fn a_part_put_again_is_its_next_revision_and_an_answer_to_the_one_he_read_is_a_409() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.add_part(None, words(&id, "Plan", "steps", "First.")).await.unwrap();
    let first = artifacts(&rig, &id).await[0].parts[0].clone();
    rig.fleet.add_part(None, words(&id, "Plan", "steps", "Second.")).await.unwrap();

    let held = artifacts(&rig, &id).await;
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].parts.len(), 1, "the same id is one part");
    assert_eq!(held[0].parts[0].revision, 2);
    let Err(Refusal::IllegalMove(stale)) = Arc::clone(&rig.fleet).answer_part(answer(&id, &held[0].id, "steps", &first.drawn, PartHow::Approved)).await
    else {
        panic!("the revision he read was replaced");
    };
    assert_eq!(stale.code, "fleet.part_redrawn");
    assert!(told(&rig.rows(&id).await).is_empty(), "the Job is told nothing");
}

#[tokio::test]
async fn a_drawing_retitled_with_against_is_the_earlier_ones_next_revision_and_leaves_one_item() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Cache design", None)).await.unwrap();
    let mut again = drawn(&id, "Cache design, with eviction", Some("Cache design"));
    again.scene = ipc::decode("a scene", TAKE_A.as_bytes()).unwrap();
    rig.fleet.show_sketch(None, again).await.unwrap();

    let held = artifacts(&rig, &id).await;
    assert_eq!(held.len(), 1, "{:?}", held.iter().map(|one| &one.id).collect::<Vec<_>>());
    assert_eq!(held[0].revisions.len(), 2, "the new drawing is a revision of the earlier one");
    assert_eq!(his_list(&rig, &id).await.len(), 1, "one item, not two");
}

#[tokio::test]
async fn a_sketch_then_a_draft_then_a_page_shown_while_the_first_is_open_are_one_artifact_of_three_parts_and_one_item() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow", None)).await.unwrap();
    rig.fleet
        .propose_draft(None, ipc::ProposeDraft { title: "Description for #41".into(), text: "Words.".into(), session_id: Some(id.clone()), ..Default::default() })
        .await
        .unwrap();
    asking_page(&rig, &id, at).await;

    let held = artifacts(&rig, &id).await;
    assert_eq!(held.len(), 1, "one artifact: {:?}", held.iter().map(|one| (&one.id, one.parts.len())).collect::<Vec<_>>());
    let kinds: Vec<PartKind> = held[0].parts.iter().map(|part| part.content.kind()).collect();
    assert_eq!(kinds, [PartKind::Drawing, PartKind::Words, PartKind::Page]);
    assert_eq!(held[0].state, ArtifactState::Asking);

    // What is shown in a turn still running waits once that turn ends.
    rig.stand_in.finishes(at, "Shown.");
    eventually(|| async { !his_list(&rig, &id).await.is_empty() }).await;
    let list = his_list(&rig, &id).await;
    assert_eq!(list.len(), 1, "one Needs you item: {list:?}");
    assert_eq!(list[0].0, held[0].id);
    for name in ["Request flow", "Description for #41", "Lease expiry"] {
        assert!(list[0].1.contains(name), "it names {name}: {}", list[0].1);
    }

    // Once every ask is answered it settles, and the next thing the Job shows is a new artifact.
    let item = ipc::ItemAnswer { session_id: id.clone(), item: held[0].id.clone(), how: ipc::ItemHow::Approve, notes: Vec::new(), extras: Default::default() };
    let settled = Arc::clone(&rig.fleet).answer_items(ipc::AnswerItems { items: vec![item] }, Vec::new()).await;
    assert_eq!(settled.outcomes.len(), 1);
    assert!(his_list(&rig, &id).await.is_empty());
    assert_eq!(artifacts(&rig, &id).await[0].state, ArtifactState::Approved);
    rig.fleet.show_sketch(None, drawn(&id, "Next thing", None)).await.unwrap();
    let held = artifacts(&rig, &id).await;
    assert_eq!(held.len(), 2, "a settled artifact starts nothing: the next sketch is its own");
    assert_eq!(his_list(&rig, &id).await.len(), 1);
}

#[tokio::test]
async fn the_job_archiving_an_artifact_that_asks_withdraws_its_asks_and_his_own_archive_does_not() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Cache design", None)).await.unwrap();
    rig.fleet.add_part(None, words(&id, "Cache design", "notes", "Evict by age.")).await.unwrap();

    let archived = Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Agent, named(&id, "sketch:Cache design")).await.expect("archived");
    assert_eq!((archived.state, archived.item.as_deref()), (ArtifactState::Withdrawn, None), "its asks are taken back, not left asking");
    assert!(archived.parts.iter().all(|part| part.state == PartState::Withdrawn), "{:?}", archived.parts);
    assert!(his_list(&rig, &id).await.is_empty());
    assert!(told(&rig.rows(&id).await).is_empty(), "the Job is told nothing of its own act");

    // His own Archive leaves the asks as they stand.
    let other = rig_placing(None);
    let mine = other.start().await;
    other.fleet.show_sketch(None, drawn(&mine, "Cache design", None)).await.unwrap();
    let his = Arc::clone(&other.fleet).archive_artifact(None, ArtifactBy::Owner, named(&mine, "sketch:Cache design")).await.expect("archived");
    assert_eq!(his.state, ArtifactState::Asking, "as it stood");
    assert_eq!(his.archived.map(|one| one.by), Some(ArtifactBy::Owner));
}

#[tokio::test]
async fn the_job_archiving_a_part_that_still_asks_withdraws_that_part() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow", None)).await.unwrap();
    let path = asking_page(&rig, &id, at).await;
    rig.stand_in.finishes(at, "Shown.");
    eventually(|| async { !his_list(&rig, &id).await.is_empty() }).await;

    // The page is a part of the artifact and is still named by its own path.
    let archived = Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Agent, named(&id, &path)).await.expect("archived");
    assert_eq!(archived.state, ArtifactState::Withdrawn);
    let held = artifacts(&rig, &id).await;
    assert_eq!(held.len(), 1);
    let list = his_list(&rig, &id).await;
    assert_eq!(list.len(), 1, "the drawing still asks him: {list:?}");
    assert!(!list[0].1.contains("Lease expiry"), "the withdrawn page is not named: {}", list[0].1);
}

/// A Job whose `src/a.css` sets the token its takes' mocks draw with, and a takes part of two mocks put on it.
async fn takes_of_mocks(rig: &Rig) -> (SessionId, ipc::SessionRecord) {
    let id = rig.start().await;
    let css = std::path::Path::new(&rig.root).join("src/a.css");
    std::fs::create_dir_all(css.parent().unwrap()).unwrap();
    std::fs::write(&css, ":root { --bg-raised: #222 } .a { background: var(--bg-raised) }").unwrap();
    let mock = |title: &str| {
        format!(r#"{{"id":"{title}","title":"{title}","scene":{{"nodes":[{{"id":"m","kind":"mock","x":0,"y":0,"w":300,"html":"<b class=\"a\">x</b>","styles":["src/a.css"]}}],"edges":[]}}}}"#)
    };
    let part = format!(r#""title":"Layout","part":{{"id":"layout","kind":"takes","takes":[{},{}]}}"#, mock("left"), mock("right"));
    let record = rig.fleet.add_part(None, added(&id, &part)).await.unwrap();
    (id, record)
}

/// The parts as the artifact's row keeps them, read off `record` and not through `get_job_artifacts`.
fn parts_kept(record: &ipc::SessionRecord) -> (&ipc::Attachment, Vec<ipc::ArtifactPart>) {
    let row = record.attachments.iter().find(|one| one.detail.contains_key(ipc::ARTIFACT_PARTS)).expect("the artifact's row");
    (row, ipc::decode("an artifact's parts", row.detail[ipc::ARTIFACT_PARTS].as_bytes()).unwrap())
}

/// Each take's mock in `parts` carries `kept`, naming a style set that holds the app's token.
async fn takes_kept(rig: &Rig, parts: &[ipc::ArtifactPart]) {
    let Some(ipc::PartContent::Takes { takes, .. }) = parts.iter().map(|part| &part.content).find(|content| content.kind() == PartKind::Takes) else {
        panic!("no takes part: {parts:?}")
    };
    for take in takes {
        let scene: ipc::Scene = ipc::decode("a take's scene", take.scene.as_deref().expect("a scene").as_bytes()).unwrap();
        let kept = scene.nodes[0].kept.clone().unwrap_or_else(|| panic!("take `{}` has no `kept`, so native draws it without the app's styles", take.id));
        assert!(rig.fleet.get_kept_styles(kept).await.unwrap().sheets[0].css.contains("--bg-raised"));
    }
}

#[tokio::test]
async fn a_takes_part_keeps_each_mock_with_its_style_set_so_it_draws_in_the_apps_styles() {
    let rig = rig_placing(None);
    let (_, record) = takes_of_mocks(&rig).await;
    takes_kept(&rig, &parts_kept(&record).1).await;
}

#[tokio::test]
async fn takes_kept_before_their_mocks_were_styled_are_styled_the_first_time_they_are_read() {
    let rig = rig_placing(None);
    let (id, record) = takes_of_mocks(&rig).await;
    // The row as a Fleet before the fix kept it: each take's mock with no `kept`.
    let (row, mut parts) = parts_kept(&record);
    for part in parts.iter_mut() {
        if let ipc::PartContent::Takes { takes, .. } = &mut part.content {
            for take in takes.iter_mut() {
                let scene: ipc::Scene = ipc::decode("a take's scene", take.scene.as_deref().unwrap().as_bytes()).unwrap();
                let hash = scene.nodes[0].kept.clone().expect("kept by add_part");
                take.scene = take.scene.take().map(|text| text.replace(&format!(r#","kept":"{hash}""#), "").replace(&format!(r#""kept":"{hash}","#), ""));
                assert!(!take.scene.as_deref().unwrap().contains(&hash), "the hash is taken off: {:?}", take.scene);
            }
        }
    }
    let mut detail = row.detail.clone();
    detail.insert(ipc::ARTIFACT_PARTS.into(), ipc::encode(&parts).unwrap());
    let attachment = ipc::AttachmentReport { kind: row.kind.clone(), target: row.target.clone(), detail };
    rig.fleet
        .report_session(ipc::SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact: ipc::SessionFact::Attached { attachment } })
        .await
        .expect("held as before the fix");

    let held = artifacts(&rig, &id).await;
    takes_kept(&rig, &held[0].parts).await;
    let (_, parts) = parts_kept(&rig.fleet.published_hosted(id.as_str()).await.unwrap());
    takes_kept(&rig, &parts).await;
}
