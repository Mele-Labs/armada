//! The claim, written before the code: **a Job writes a scene and its fixtures under `scenes/` in its scratch folder
//! and shows it; Fleet reads them into a prototype part, and each save of one of those files puts the part again as
//! its next revision, with the state the Job set.** `crates/ipc/operations/show_prototype.toml`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use api::{HostedSessions, Next, Refusal, Sessions};
use ipc::{ArtifactPart, Event, PartContent, PartKind, PartState, PrototypePart, SessionFact, SessionId, SessionReport, ShowPrototype};

use super::session_host::{rig_placing, Rig};

const SCENE: &str = r#"<scene><fixture name="jobs" src="jobs.json"/><state name="empty"/><state name="full"/></scene>"#;

fn shown(id: &SessionId, scene: &str, state: Option<&str>) -> ShowPrototype {
    ShowPrototype { title: String::from("Ready screen"), scene: scene.into(), state: state.map(str::to_string), for_line: None, session_id: Some(id.clone()) }
}

/// The folder the Job's scenes are written in, as `$ARMADA_SCRATCH/scenes`.
fn scenes_of(rig: &Rig, id: &SessionId) -> PathBuf {
    let folder = rig.fleet.scratch_of(id.as_str()).join("scenes");
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn written(folder: &Path, name: &str, text: &str) -> String {
    std::fs::write(folder.join(name), text).unwrap();
    folder.join(name).to_string_lossy().into_owned()
}

async fn prototype(rig: &Rig, id: &SessionId) -> (ArtifactPart, PrototypePart) {
    let part = rig.fleet.get_job_artifacts(id.clone()).await.unwrap().into_iter().flat_map(|one| one.parts).find(|part| part.id == "ready.html").expect("the prototype part");
    let PartContent::Prototype(held) = part.content.clone() else { panic!("a prototype part, not {:?}", part.content.kind()) };
    (part, held)
}

async fn saved(rig: &Rig, id: &SessionId, paths: Vec<String>) {
    rig.fleet
        .report_session(SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact: SessionFact::Touched { paths, by: None, act: None } })
        .await
        .expect("reported");
}

#[tokio::test]
async fn a_scene_named_as_a_path_is_refused_and_nothing_is_written() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let folder = scenes_of(&rig, &id);
    written(&folder, "ready.html", SCENE);
    for name in ["scenes/ready.html", "../ready.html", "/etc/hosts", "..", ".hidden.html", "a\\b.html", ""] {
        let Err(Refusal::Unacceptable(why)) = rig.fleet.show_prototype(None, shown(&id, name, None)).await else { panic!("`{name}` is refused") };
        assert_eq!(why.code, "fleet.part_refused", "{name}");
        assert!(why.message.contains("`scene`"), "it names the field: {}", why.message);
    }
    assert!(rig.fleet.get_job_artifacts(id.clone()).await.unwrap().is_empty(), "no artifact was made");
}

#[tokio::test]
async fn a_scene_and_the_fixtures_it_names_are_read_into_a_part_that_asks() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let folder = scenes_of(&rig, &id);
    written(&folder, "ready.html", SCENE);
    written(&folder, "jobs.json", r#"[{"id":1}]"#);
    written(&folder, "other.json", "[]");

    rig.fleet.show_prototype(None, shown(&id, "ready.html", Some("full"))).await.expect("shown");

    let (part, held) = prototype(&rig, &id).await;
    assert_eq!((part.content.kind(), part.asks, part.state, part.revision), (PartKind::Prototype, true, PartState::Open, 1));
    assert_eq!(held.markup, SCENE);
    assert_eq!(held.fixtures.iter().map(|one| (one.name.as_str(), one.text.as_str())).collect::<Vec<_>>(), [("jobs.json", r#"[{"id":1}]"#)], "only the fixture it names");
    assert_eq!((held.scene_state.as_deref(), held.trouble), (Some("full"), None));
}

#[tokio::test]
async fn a_missing_fixture_and_a_scene_past_its_cap_are_refused_saying_which() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let folder = scenes_of(&rig, &id);
    written(&folder, "ready.html", SCENE);
    let Err(Refusal::Unacceptable(missing)) = rig.fleet.show_prototype(None, shown(&id, "ready.html", None)).await else { panic!("a fixture is missing") };
    assert!(missing.message.contains("scenes/jobs.json") && missing.message.contains("not there"), "{}", missing.message);
    written(&folder, "big.html", &"x".repeat(ipc::MOST_SCENE_BYTES + 1));
    let Err(Refusal::Unacceptable(big)) = rig.fleet.show_prototype(None, shown(&id, "big.html", None)).await else { panic!("a scene past 64 KB") };
    assert!(big.message.contains("scenes/big.html") && big.message.contains("weighs"), "{}", big.message);
}

#[tokio::test]
async fn a_save_of_the_scene_or_a_fixture_is_the_next_revision_and_keeps_the_state() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let folder = scenes_of(&rig, &id);
    let scene = written(&folder, "ready.html", SCENE);
    let fixture = written(&folder, "jobs.json", "[1]");
    rig.fleet.show_prototype(None, shown(&id, "ready.html", Some("full"))).await.expect("shown");
    let (first, _) = prototype(&rig, &id).await;
    let mut watching = rig.fleet.events().subscribe();

    written(&folder, "ready.html", &format!("{SCENE}<!-- v2 -->"));
    saved(&rig, &id, vec![scene]).await;
    let (second, held) = prototype(&rig, &id).await;
    assert_eq!((second.revision, held.scene_state.as_deref()), (2, Some("full")), "the scene is put again with the state it had");
    assert!(held.markup.ends_with("<!-- v2 -->"), "{}", held.markup);
    assert_ne!(second.drawn, first.drawn, "a new revision is a new `drawn`");

    written(&folder, "jobs.json", "[1,2]");
    saved(&rig, &id, vec![fixture]).await;
    let (third, held) = prototype(&rig, &id).await;
    assert_eq!((third.revision, held.fixtures[0].text.as_str()), (3, "[1,2]"));

    let published = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if matches!(delivered.event, Event::SessionChanged(_)) {
                        return;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await;
    assert!(published.is_ok(), "a save signals session.changed");
}

#[tokio::test]
async fn a_save_to_a_file_the_scene_does_not_name_changes_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let folder = scenes_of(&rig, &id);
    written(&folder, "ready.html", SCENE);
    written(&folder, "jobs.json", "[1]");
    rig.fleet.show_prototype(None, shown(&id, "ready.html", None)).await.expect("shown");

    let other = written(&folder, "other.json", "[]");
    let notes = written(&rig.fleet.scratch_of(id.as_str()), "notes.txt", "scratch");
    saved(&rig, &id, vec![other, notes, String::from("src/lib.rs")]).await;
    assert_eq!(prototype(&rig, &id).await.0.revision, 1);

    // Nor does a save that leaves the files reading as the part does.
    saved(&rig, &id, vec![folder.join("ready.html").to_string_lossy().into_owned()]).await;
    assert_eq!(prototype(&rig, &id).await.0.revision, 1);
}

#[tokio::test]
async fn a_save_that_cannot_be_read_is_the_trouble_and_the_last_text_stays() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let folder = scenes_of(&rig, &id);
    let scene = written(&folder, "ready.html", SCENE);
    written(&folder, "jobs.json", "[1]");
    rig.fleet.show_prototype(None, shown(&id, "ready.html", None)).await.expect("shown");

    std::fs::remove_file(folder.join("jobs.json")).unwrap();
    saved(&rig, &id, vec![scene]).await;
    let (part, held) = prototype(&rig, &id).await;
    assert_eq!(part.revision, 2);
    assert!(held.trouble.as_deref().is_some_and(|why| why.contains("scenes/jobs.json")), "{:?}", held.trouble);
    assert_eq!(held.fixtures.len(), 1, "the last good fixture is still drawn");
}
