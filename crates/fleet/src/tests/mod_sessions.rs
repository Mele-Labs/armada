//! `start_mod_session`: a Mod Job on Armada's own repository, or in the mods folder where it is not
//! served, sent the Mod brief and what the person pointed at as one first message, and the mods it
//! scaffolds listing it as `made_by`.

use std::sync::Arc;

use api::{HostedSessions, Mods, Refusal, Sessions};
use ipc::{ModKind, ScaffoldMod, SessionId, SessionRow, StartModSession};

use super::session_dispute::serving;
use super::session_host::{denied, Rig};
use super::session_notes::{a_repository, note, turn_of, PATH, SAID};

fn pointed(title: Option<&str>) -> StartModSession {
    let at = note(&SessionId::carried("unused"), "Other commands, 16.1% of the burn", None).note;
    StartModSession { title: title.map(str::to_string), model: None, effort: None, note: Some(at), said: None }
}

fn spoken(title: Option<&str>, said: Option<&str>) -> StartModSession {
    StartModSession {
        title: title.map(str::to_string),
        model: None,
        effort: None,
        note: None,
        said: said.map(str::to_string),
    }
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
async fn a_mod_job_starts_in_the_mods_folder_where_fleet_does_not_serve_armada() {
    let rig = serving("elsewhere").await;
    let mods = rig.fleet.mods_dir().to_string_lossy().into_owned();

    let started = started(&rig, None).await;

    assert_eq!(started.send_failed, None);
    let record = Arc::clone(&rig.fleet).get_session(started.session_id.clone()).await.expect("kept").session;
    assert!(record.r#mod, "marked as a Mod Job: {record:#?}");
    assert_eq!((record.manifest_id.as_ref(), record.cwd.as_str()), (None, mods.as_str()), "{record:#?}");
    let turn = turn_of(&rig).await;
    assert_eq!(rig.stand_in.starts()[0].directory, mods, "the process runs in the mods folder");
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [at("This is a Mod Job"), at("armada:armada-mods"), at("Armada's source is not here"), at(SAID)];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "the mods brief, then his words: {turn}");
    assert!(!turn.contains("scripts/preview --restart"), "no source to bring in: {turn}");

    let written = format!(r#"{{"file_path":"{mods}/calmer/tweak.css","content":"x"}}"#);
    let answer = rig.gate_in(&mods, &started.session_id, "Write", &written).await;
    assert!(!denied(&answer), "a write in the mods folder runs: {answer:?}");
    let hosting = rig.fleet.store().lock().await.hosting(started.session_id.as_str()).expect("read").expect("hosted");
    assert_eq!((hosting.lease_slot, hosting.lease_branch.as_deref()), (None, None), "no slot and no branch");
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
async fn a_mod_job_started_from_his_words_is_sent_the_brief_then_them() {
    let rig = serving("armada").await;
    let words = "  make the usage rows calmer\nand quieter  ";

    let started = Arc::clone(&rig.fleet).start_mod_session(spoken(None, Some(words))).await.expect("started");

    assert_eq!((started.title.as_str(), started.send_failed.as_deref()), ("make the usage rows calmer", None));
    let record = Arc::clone(&rig.fleet).get_session(started.session_id.clone()).await.expect("kept").session;
    assert!(record.r#mod, "marked as a Mod Job: {record:#?}");
    assert_eq!(record.manifest_id.as_ref().map(|id| id.as_str()), Some("armada"));
    let turn = turn_of(&rig).await;
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [
        at("This is a Mod Job"),
        at("armada-mods"),
        at("scripts/preview --restart"),
        at("make the usage rows calmer"),
        at("and quieter"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "the brief, then his words: {turn}");
    assert!(!turn.contains("Pointed at"), "nothing was pointed at: {turn}");

    let rows = rig.rows(&started.session_id).await;
    assert!(
        matches!(&rows[..], [SessionRow::Message { text, .. }] if text == words.trim()),
        "the thread keeps his words alone: {rows:#?}"
    );
}

#[tokio::test]
async fn a_mod_job_started_from_his_words_in_the_mods_folder_is_sent_the_mods_brief_then_them() {
    let rig = serving("elsewhere").await;
    let mods = rig.fleet.mods_dir().to_string_lossy().into_owned();

    let started = Arc::clone(&rig.fleet)
        .start_mod_session(spoken(Some("Calmer rows"), Some("a quieter theme")))
        .await
        .expect("started");

    assert_eq!((started.title.as_str(), started.send_failed.as_deref()), ("Calmer rows", None));
    let turn = turn_of(&rig).await;
    assert_eq!(rig.stand_in.starts()[0].directory, mods, "the process runs in the mods folder");
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [at("This is a Mod Job in the mods folder"), at("Armada's source is not here"), at("a quieter theme")];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "the mods brief, then his words: {turn}");
}

#[tokio::test]
async fn a_mod_start_naming_neither_his_words_nor_a_note_or_both_is_refused_and_starts_nothing() {
    let rig = serving("armada").await;
    let both = StartModSession { note: pointed(None).note, ..spoken(None, Some("calmer rows")) };
    let blank = |said: &str| spoken(None, Some(said));

    for (what, start) in [
        ("neither", spoken(None, None)),
        ("both", both),
        ("empty words", blank("")),
        ("blank words", blank(" \n\t ")),
    ] {
        let refused = Arc::clone(&rig.fleet).start_mod_session(start).await;
        assert!(
            matches!(&refused, Err(Refusal::Unacceptable(why)) if why.code == "fleet.unacceptable_mod_start"),
            "{what}: {refused:?}"
        );
    }

    assert!(rig.stand_in.starts().is_empty(), "no process was started");
    let listed = rig.fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    assert!(listed.is_empty(), "no Job was made: {listed:#?}");
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
