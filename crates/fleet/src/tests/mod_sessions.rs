//! `start_mod_session`: a Mod Job on Armada's own repository, sent the Mod brief and what the person
//! pointed at as one first message, and the mods it scaffolds listing it as `made_by`.

use std::sync::Arc;

use api::{HostedSessions, Mods, Refusal};
use ipc::{ModKind, ScaffoldMod, SessionId, SessionRow, StartModSession};

use super::session_dispute::serving;
use super::session_host::Rig;
use super::session_notes::{a_repository, note, turn_of, PATH, SAID};

fn pointed(title: Option<&str>) -> StartModSession {
    let at = note(&SessionId::carried("unused"), "Other commands, 16.1% of the burn", None).note;
    StartModSession { title: title.map(str::to_string), model: None, effort: None, note: at }
}

async fn started(rig: &Rig, title: Option<&str>) -> ipc::ModSessionStarted {
    Arc::clone(&rig.fleet).start_mod_session(pointed(title)).await.expect("started")
}

#[tokio::test]
async fn a_mod_job_starts_on_armadas_own_repository_marked_mod() {
    let rig = serving("armada").await;

    let started = started(&rig, Some("Calmer usage rows")).await;

    assert_eq!(started.title, "Calmer usage rows");
    let record = Arc::clone(&rig.fleet).get_session(started.session_id.clone()).await.expect("kept").session;
    assert!(record.r#mod, "marked as a Mod Job: {record:#?}");
    assert_eq!(record.manifest_id.as_ref().map(|id| id.as_str()), Some("armada"));
    assert_eq!(record.title.as_deref(), Some("Calmer usage rows"));
}

#[tokio::test]
async fn a_mod_job_is_refused_where_fleet_does_not_serve_armada() {
    let rig = serving("elsewhere").await;

    let refused = Arc::clone(&rig.fleet).start_mod_session(pointed(None)).await;

    let Err(Refusal::Unacceptable(why)) = refused else { panic!("refused as a 422: {refused:?}") };
    assert_eq!(why.code, "fleet.mod_armada_not_served");
    assert!(rig.stand_in.starts().is_empty(), "nothing is started");
}

#[tokio::test]
async fn a_mod_jobs_first_message_is_the_brief_then_what_he_pointed_at() {
    let rig = serving("armada").await;
    a_repository(&rig);

    let started = started(&rig, None).await;

    assert_eq!((started.title.as_str(), started.send_failed.as_deref()), (SAID, None));
    let turn = turn_of(&rig).await;
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [
        at("This is a Mod Job"),
        at("armada-mods"),
        at("scripts/preview --restart"),
        at(SAID),
        at("Pointed at: li \\\"Other commands, 16.1% of the burn\\\""),
        at(&format!("Candidates:\\n1. {PATH}:171 — ")),
        at("Inside: Burn by Bash command; component chain ShareRows ← UsagePage ← Boundary"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "the brief, then his words, then the spot: {turn}");

    let rows = rig.rows(&started.session_id).await;
    assert!(
        matches!(&rows[..], [SessionRow::Message { text, .. }] if text == SAID),
        "the thread keeps his words alone: {rows:#?}"
    );
}

#[tokio::test]
async fn a_mod_a_job_scaffolds_lists_that_job_as_made_by() {
    let rig = serving("armada").await;
    let job = rig.start().await;
    let scaffold = |name: &str, by: Option<SessionId>| ScaffoldMod {
        name: name.to_string(),
        description: None,
        kind: Some(ModKind::Tweak),
        session_id: by,
    };

    rig.fleet.scaffold_mod(scaffold("calmer", Some(job.clone()))).await.expect("made");
    rig.fleet.scaffold_mod(scaffold("by-hand", None)).await.expect("made");
    let stray = rig.fleet.scaffold_mod(scaffold("stray", Some(SessionId::carried("01NOBODY")))).await;
    assert!(matches!(&stray, Err(Refusal::Unacceptable(why)) if why.code == "fleet.unacceptable_mod"), "{stray:?}");

    let list = rig.fleet.list_mods().await.expect("listed").mods;
    let made_by = |name: &str| list.iter().find(|one| one.name == name).and_then(|one| one.made_by.clone());
    assert_eq!(made_by("calmer"), Some(job));
    assert_eq!(made_by("by-hand"), None, "a call that names no Job records none");
    assert!(list.iter().all(|one| one.name != "stray"), "an unknown Job makes nothing");
}
