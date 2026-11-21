//! The claim, written before the code: **one ask is one item.** A sketch, a draft, a page and a part shown while
//! another still asks are each their own artifact and their own Needs you item, and nothing joins another on its
//! own. A part goes on the artifact of the title it names, he answers each part that asks on its own, and the Job is
//! told each answer naming the artifact and the part. A retitled drawing is the earlier one's next revision, and an
//! artifact the Job archives while it asks has its asks withdrawn.
//! `docs/decisions` *one ask, one item, one place*; `crates/ipc/operations/add_part.toml`, `answer_part.toml`,
//! `docs/concepts/artifact.md`, *Parts*.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use ipc::{AddPart, AnswerPart, ArtifactBy, ArtifactRef, ArtifactState, PartHow, PartKind, PartState, SessionId, ShowSketch};

use super::artifacts::a_job;
use super::session_host::{eventually, rig_placing, Rig};

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;
const TAKE_A: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Left"}],"edges":[]}"#;
const TAKE_B: &str = r#"{"nodes":[{"id":"m","kind":"box","x":0,"y":0,"title":"Right"}],"edges":[]}"#;

fn drawn(id: &SessionId, title: &str, against: Option<&str>) -> ShowSketch {
    let against = against.map(|one| format!(r#","against":"{one}""#)).unwrap_or_default();
    ipc::decode("a sketch", format!(r#"{{"title":"{title}","question":"Does this read right?","session_id":"{}","scene":{SCENE}{against}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn added(id: &SessionId, body: &str) -> AddPart {
    ipc::decode("a part", format!(r#"{{"session_id":"{}",{body}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn words(id: &SessionId, title: &str, part: &str, text: &str) -> AddPart {
    added(id, &format!(r#""title":"{title}","question":"Is {part} right?","part":{{"id":"{part}","kind":"words","text":"{text}"}}"#))
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
    // The page is kept, and named, at its copy in Fleet's folder.
    rig.fleet.page_home(id.as_str(), &path)
}

/// The refusal `said` must be, as its code and message.
fn refused<T: std::fmt::Debug>(said: Result<T, Refusal>) -> ipc::WireError {
    match said {
        Err(Refusal::Unacceptable(why)) => why,
        other => panic!("refused, not {other:?}"),
    }
}

/// `part` added with `asks` false: an aside that asks nothing.
fn aside(mut part: AddPart) -> AddPart {
    part.asks = Some(false);
    part
}

#[tokio::test]
async fn one_artifact_asks_one_thing_and_holds_asides_and_buttons_beside_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Cache design", None)).await.unwrap();

    // A second ask on it is refused, naming the ask it holds; an aside and buttons go on.
    let second = refused(rig.fleet.add_part(None, words(&id, "Cache design", "notes", "Evict by age.")).await);
    assert_eq!(second.code, "fleet.part_refused");
    assert!(second.message.contains("one artifact asks one thing") && second.message.contains("`Cache design`"), "{}", second.message);
    rig.fleet.add_part(None, aside(words(&id, "Cache design", "notes", "Evict by age."))).await.unwrap();
    rig.fleet.add_part(None, buttons(&id, "Cache design")).await.unwrap();

    let held = artifacts(&rig, &id).await;
    assert_eq!(held.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["sketch:Cache design"]);
    let parts: Vec<(&str, PartKind, bool)> = held[0].parts.iter().map(|part| (part.id.as_str(), part.content.kind(), part.asks)).collect();
    assert_eq!(parts, [("sketch:Cache design", PartKind::Drawing, true), ("notes", PartKind::Words, false), ("go", PartKind::Buttons, false)]);
    assert_eq!(his_list(&rig, &id).await, [("sketch:Cache design".to_string(), "Does this read right?".to_string())], "one item, titled with its one ask");

    // The aside has nothing to answer, and the drawing's answer settles the whole.
    let notes = held[0].parts.iter().find(|part| part.id == "notes").unwrap();
    let informs = refused(Arc::clone(&rig.fleet).answer_part(answer(&id, "sketch:Cache design", "notes", &notes.drawn, PartHow::Approved)).await);
    assert_eq!(informs.code, "fleet.part_not_open");
    let drawing = held[0].parts.iter().find(|part| part.content.kind() == PartKind::Drawing).unwrap();
    let done = Arc::clone(&rig.fleet).answer_part(answer(&id, "sketch:Cache design", "sketch:Cache design", &drawing.drawn, PartHow::Approved)).await.unwrap();
    assert_eq!((done.state, done.item.as_deref()), (ArtifactState::Approved, None));
    assert!(his_list(&rig, &id).await.is_empty());

    // Answered, the drawing is not answered twice.
    let again = refused(Arc::clone(&rig.fleet).answer_part(answer(&id, "sketch:Cache design", "sketch:Cache design", &drawing.drawn, PartHow::Approved)).await);
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
async fn takes_stand_alone_so_a_drawing_or_another_part_beside_them_is_refused_whichever_comes_first() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    // A drawing first, then takes under its title: refused, saying where the background goes.
    rig.fleet.show_sketch(None, drawn(&id, "Catalog", None)).await.unwrap();
    let onto_drawing = refused(rig.fleet.add_part(None, takes(&id, "Catalog")).await);
    assert_eq!(onto_drawing.code, "fleet.part_refused");
    assert!(onto_drawing.message.contains("holds a drawing") && onto_drawing.message.contains("`question`"), "{}", onto_drawing.message);
    let mut quiet = drawn(&id, "Catalog notes", None);
    quiet.asks = false;
    rig.fleet.show_sketch(None, quiet).await.unwrap();
    assert!(refused(rig.fleet.add_part(None, takes(&id, "Catalog notes")).await).message.contains("holds a drawing"), "a drawing that informs is refused too");

    // Takes first: no drawing, words, aside or buttons goes beside them.
    rig.fleet.add_part(None, takes(&id, "Layouts")).await.unwrap();
    let drawing = refused(rig.fleet.show_sketch(None, drawn(&id, "Layouts", None)).await);
    assert_eq!(drawing.code, "fleet.sketch_takes");
    assert!(drawing.message.contains("holds takes"), "{}", drawing.message);
    let mut informing = drawn(&id, "Layouts", None);
    informing.asks = false;
    assert_eq!(refused(rig.fleet.show_sketch(None, informing).await).code, "fleet.sketch_takes");
    for part in [words(&id, "Layouts", "notes", "Why."), aside(words(&id, "Layouts", "notes", "Why.")), buttons(&id, "Layouts")] {
        let beside = refused(rig.fleet.add_part(None, part).await);
        assert!(beside.message.contains("holds the takes part `layout`"), "{}", beside.message);
    }
    // The takes put again under their own id are their next revision.
    rig.fleet.add_part(None, takes(&id, "Layouts")).await.unwrap();

    let held = artifacts(&rig, &id).await;
    let layouts = held.iter().find(|one| one.id == "sketch:Layouts").unwrap();
    assert_eq!(layouts.parts.iter().map(|part| (part.id.as_str(), part.revision)).collect::<Vec<_>>(), [("layout", 2)]);
}

#[tokio::test]
async fn takes_are_answered_by_one_pick_and_approve_of_the_whole_never_stands_in_for_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.add_part(None, takes(&id, "Catalog")).await.unwrap();
    assert_eq!(his_list(&rig, &id).await.iter().map(|(item, _)| item.as_str()).collect::<Vec<_>>(), ["sketch:Catalog"]);

    let layout = artifacts(&rig, &id).await[0].parts[0].clone();
    let wrong = refused(Arc::clone(&rig.fleet).answer_part(answer(&id, "sketch:Catalog", "layout", &layout.drawn, PartHow::Approved)).await);
    assert_eq!(wrong.code, "fleet.part_how");
    assert!(wrong.message.contains("send how `picked`"), "it says what to send: {}", wrong.message);
    let item = ipc::ItemAnswer { session_id: id.clone(), item: "sketch:Catalog".into(), how: ipc::ItemHow::Approve, notes: Vec::new(), extras: Default::default() };
    let whole = Arc::clone(&rig.fleet).answer_items(ipc::AnswerItems { items: vec![item] }, Vec::new()).await;
    let ipc::ItemSettled::Refused { message, .. } = &whole.outcomes[0].outcome else { panic!("an Approve of the whole is refused while a pick is open") };
    assert!(message.contains("send a pick"), "{message}");

    let done = picked(&rig, &id, "sketch:Catalog").await.unwrap();
    assert_eq!((done.state, done.item.as_deref()), (ArtifactState::Approved, None));
    assert_eq!(done.parts[0].state, PartState::Picked);
    assert!(his_list(&rig, &id).await.is_empty(), "one pick settles it");
    let said = told(&rig.rows(&id).await);
    assert!(said.iter().any(|one| one.contains("layout") && one.contains("Right")), "the pick is told naming the part and the take: {said:?}");
}

#[tokio::test]
async fn a_sketch_shown_with_asks_false_never_waits_on_him() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut quiet = drawn(&id, "Request flow", None);
    quiet.asks = false;
    rig.fleet.show_sketch(None, quiet.clone()).await.unwrap();
    rig.fleet.show_sketch(None, quiet).await.unwrap();
    assert!(his_list(&rig, &id).await.is_empty(), "drawn and redrawn, it informs");
    let held = artifacts(&rig, &id).await;
    assert_eq!(held.len(), 1, "it is kept in Artifacts");
    assert!(held[0].parts.iter().all(|part| !part.asks), "{held:?}");
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
async fn a_sketch_a_draft_a_page_and_a_part_shown_while_the_first_still_asks_are_each_their_own_artifact_and_item() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow", None)).await.unwrap();
    rig.fleet
        .propose_draft(None, ipc::ProposeDraft { title: "Description for #41".into(), text: "Words.".into(), session_id: Some(id.clone()), ..Default::default() })
        .await
        .unwrap();
    let path = asking_page(&rig, &id, at).await;
    rig.fleet.add_part(None, words(&id, "Cache notes", "notes", "Evict by age.")).await.unwrap();

    // Nothing joins the sketch that still asks: four artifacts, each of one part.
    let held = artifacts(&rig, &id).await;
    let mut ids: Vec<&str> = held.iter().map(|one| one.id.as_str()).collect();
    ids.sort();
    let mut four = ["draft:Description for #41", path.as_str(), "sketch:Cache notes", "sketch:Request flow"];
    four.sort();
    assert_eq!(ids, four, "{held:?}");
    assert!(held.iter().all(|one| one.parts.len() <= 1 && one.state == ArtifactState::Asking), "{held:?}");

    // What is shown in a turn still running waits once that turn ends, each as its own item.
    rig.stand_in.finishes(at, "Shown.");
    eventually(|| async { his_list(&rig, &id).await.len() == 4 }).await;
    let list = his_list(&rig, &id).await;
    let mut items: Vec<&str> = list.iter().map(|(item, _)| item.as_str()).collect();
    items.sort();
    let page_item = format!("{}{path}", ipc::DOCUMENT_ITEM_PREFIX);
    let mut wanted = vec![page_item.as_str(), "sketch:Cache notes", "sketch:Request flow"];
    let draft = list.iter().find(|(item, _)| item.starts_with("draft:")).map(|(item, _)| item.clone()).expect("the draft waits on its own");
    wanted.push(draft.as_str());
    wanted.sort();
    assert_eq!(items, wanted, "{list:?}");

    // Answering one leaves the others as they stand.
    let item = ipc::ItemAnswer { session_id: id.clone(), item: "sketch:Request flow".into(), how: ipc::ItemHow::Approve, notes: Vec::new(), extras: Default::default() };
    let settled = Arc::clone(&rig.fleet).answer_items(ipc::AnswerItems { items: vec![item] }, Vec::new()).await;
    assert_eq!(settled.outcomes.len(), 1);
    assert_eq!(his_list(&rig, &id).await.len(), 3);
}

#[tokio::test]
async fn the_job_archiving_an_artifact_that_asks_withdraws_its_asks_and_his_own_archive_does_not() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Cache design", None)).await.unwrap();
    // One artifact asks one thing, so the words beside its drawing only inform.
    rig.fleet.add_part(None, aside(words(&id, "Cache design", "notes", "Evict by age."))).await.unwrap();

    let archived = Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Agent, named(&id, "sketch:Cache design")).await.expect("archived");
    assert_eq!((archived.state, archived.item.as_deref()), (ArtifactState::Withdrawn, None), "its asks are taken back, not left asking");
    assert!(archived.parts.iter().filter(|part| part.asks).all(|part| part.state == PartState::Withdrawn), "{:?}", archived.parts);
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
async fn the_job_archiving_a_page_that_still_asks_withdraws_it_and_leaves_the_sketch_asking() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow", None)).await.unwrap();
    let path = asking_page(&rig, &id, at).await;
    rig.stand_in.finishes(at, "Shown.");
    eventually(|| async { his_list(&rig, &id).await.len() == 2 }).await;

    let archived = Arc::clone(&rig.fleet).archive_artifact(None, ArtifactBy::Agent, named(&id, &path)).await.expect("archived");
    assert_eq!(archived.state, ArtifactState::Withdrawn);
    let list = his_list(&rig, &id).await;
    assert_eq!(list.iter().map(|(item, _)| item.as_str()).collect::<Vec<_>>(), ["sketch:Request flow"], "the drawing still asks him: {list:?}");
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
    let part = format!(r#""title":"Layout","part":{{"id":"layout","kind":"takes","question":"Which layout?","takes":[{},{}]}}"#, mock("left"), mock("right"));
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
