//! `start_mod_session`: a Mod Job on Armada's own repository, or in the mods folder where it is not
//! served, sent the Mod brief and what the person pointed at as one first message, and the mods it
//! scaffolds listing it as `made_by`.

use std::sync::Arc;

use api::{HostedSessions, Mods, Refusal, Sessions};
use ipc::{ModAim, ModKind, ScaffoldMod, SendSessionMessage, SessionId, SessionRow, StartModSession};

use super::session_dispute::serving;
use super::session_host::{denied, eventually, Rig};
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

fn said_by_him(id: &SessionId, text: &str) -> SendSessionMessage {
    SendSessionMessage { session_id: id.clone(), text: text.to_string(), attachments: Vec::new(), mentions: Vec::new() }
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
async fn a_mod_start_naming_a_note_and_his_words_or_words_with_nothing_in_them_is_refused_and_starts_nothing() {
    let rig = serving("armada").await;
    let both = StartModSession { note: pointed(None).note, ..spoken(None, Some("calmer rows")) };
    let blank = |said: &str| spoken(None, Some(said));

    for (what, start) in [
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
async fn a_mod_job_started_on_nothing_is_empty_and_titled_mod_in_either_place() {
    for (served, in_mods) in [("armada", false), ("elsewhere", true)] {
        let rig = serving(served).await;

        let started = Arc::clone(&rig.fleet).start_mod_session(spoken(None, None)).await.expect("started");

        assert_eq!((started.title.as_str(), started.send_failed.as_deref()), ("Mod", None), "{served}");
        let record = Arc::clone(&rig.fleet).get_session(started.session_id.clone()).await.expect("kept").session;
        assert!(record.r#mod, "{served}: marked as a Mod Job: {record:#?}");
        assert_eq!(record.title.as_deref(), Some("Mod"), "{served}");
        assert_eq!(record.manifest_id.is_none(), in_mods, "{served}: {record:#?}");
        assert!(rig.rows(&started.session_id).await.is_empty(), "{served}: nothing is on the thread");
        assert!(rig.stand_in.starts().is_empty(), "{served}: nothing is sent, so no process runs");
    }
}

#[tokio::test]
async fn an_empty_mod_jobs_first_message_carries_the_brief_and_retitles_it_from_his_words() {
    let rig = serving("armada").await;
    let words = "  make the usage rows calmer and a good deal quieter than they are\nthen the theme  ";
    let id = Arc::clone(&rig.fleet).start_mod_session(spoken(None, None)).await.expect("started").session_id;

    Arc::clone(&rig.fleet).send_session_message(said_by_him(&id, words)).await.expect("sent");

    let turn = turn_of(&rig).await;
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [at("This is a Mod Job"), at("armada-mods"), at("scripts/preview --restart"), at("make the usage rows calmer")];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "the brief, then his words: {turn}");
    let rows = rig.rows(&id).await;
    assert!(
        matches!(&rows[..], [SessionRow::Message { text, .. }] if text == words.trim()),
        "the thread keeps his words alone: {rows:#?}"
    );
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("kept").session;
    assert_eq!(record.title.as_deref(), Some("Mod make the usage rows calmer and a good"), "cut at a word within 40");

    Arc::clone(&rig.fleet).send_session_message(said_by_him(&id, "and the tabs")).await.expect("sent");
    let again = Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("kept").session;
    assert_eq!(again.title, record.title, "only the first message names it");
    let second = rig.stand_in.process(0).sent.lock().unwrap().get(1).cloned();
    assert!(
        second.is_some_and(|turn| turn.contains("and the tabs") && !turn.contains("This is a Mod Job")),
        "the brief rides once"
    );
}

#[tokio::test]
async fn an_empty_mod_job_in_the_mods_folder_is_sent_the_mods_brief_and_keeps_a_title_it_was_given() {
    let rig = serving("elsewhere").await;
    let started = Arc::clone(&rig.fleet).start_mod_session(spoken(Some("Calmer rows"), None)).await.expect("started");
    assert_eq!(started.title, "Calmer rows");

    Arc::clone(&rig.fleet).send_session_message(said_by_him(&started.session_id, "a quieter theme")).await.expect("sent");

    let turn = turn_of(&rig).await;
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [at("This is a Mod Job in the mods folder"), at("Armada's source is not here"), at("a quieter theme")];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "the mods brief, then his words: {turn}");
    let record = Arc::clone(&rig.fleet).get_session(started.session_id).await.expect("kept").session;
    assert_eq!(record.title.as_deref(), Some("Calmer rows"));
}

#[tokio::test]
async fn an_empty_mod_jobs_first_message_names_what_he_is_aiming_for_between_the_brief_and_his_words() {
    let words = "calmer usage rows";
    let aims = [
        (ModAim::Theme, "He is aiming for: a theme (colours)."),
        (ModAim::Layout, "He is aiming for: a layout (which tabs and panels are drawn, and in what order)."),
        (ModAim::Words, "He is aiming for: words (a pattern Bridge links, and where it opens)."),
        (ModAim::Fleet, "He is aiming for: a Fleet rule (what Fleet does with a notice it would send)."),
        (ModAim::Tracker, "He is aiming for: a tracker (an issue tracker Armada has no adapter for)."),
        (ModAim::Commands, "He is aiming for: commands (rows in the command palette that start a Job, open an address, run a line or chain other rows)."),
        (ModAim::Source, "He is aiming for: a change to Armada's source."),
    ];
    for (aim, line) in aims {
        let rig = serving("armada").await;
        let id = Arc::clone(&rig.fleet).start_mod_session(spoken(None, None)).await.expect("started").session_id;

        Arc::clone(&rig.fleet).send_session_message_aiming(said_by_him(&id, words), Some(aim)).await.expect("sent");

        let turn = turn_of(&rig).await;
        let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("{aim:?}: the turn carries {wanted}: {turn}"));
        let order = [at("This is a Mod Job"), at(line), at(words)];
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{aim:?}: the brief, the aim, his words: {turn}");
        assert_eq!(turn.matches("He is aiming for").count(), 1, "{aim:?}: {turn}");
        let rows = rig.rows(&id).await;
        assert!(
            matches!(&rows[..], [SessionRow::Message { text, .. }] if text == words),
            "{aim:?}: the thread keeps his words alone: {rows:#?}"
        );
    }
}

#[tokio::test]
async fn a_source_aim_in_the_mods_folder_says_it_cannot_be_made_there() {
    let rig = serving("elsewhere").await;
    let id = Arc::clone(&rig.fleet).start_mod_session(spoken(None, None)).await.expect("started").session_id;

    Arc::clone(&rig.fleet).send_session_message_aiming(said_by_him(&id, "the tab bar"), Some(ModAim::Source)).await.expect("sent");

    let turn = turn_of(&rig).await;
    let line = "He is aiming for: a change to Armada's source, which cannot be made here. Tell him so if a mod cannot do it.";
    let at = |wanted: &str| turn.find(wanted).unwrap_or_else(|| panic!("the turn carries {wanted}: {turn}"));
    let order = [at("This is a Mod Job in the mods folder"), at(line), at("the tab bar")];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "the mods brief, the aim, his words: {turn}");
}

#[tokio::test]
async fn an_aim_on_any_message_but_an_empty_mod_jobs_first_is_ignored_and_not_an_error() {
    let rig = serving("armada").await;
    let id = Arc::clone(&rig.fleet).start_mod_session(spoken(None, None)).await.expect("started").session_id;
    Arc::clone(&rig.fleet).send_session_message(said_by_him(&id, "a quieter theme")).await.expect("sent");
    turn_of(&rig).await;

    Arc::clone(&rig.fleet).send_session_message_aiming(said_by_him(&id, "and the tabs"), Some(ModAim::Layout)).await.expect("taken");

    eventually(|| async { rig.stand_in.process(0).sent.lock().unwrap().len() == 2 }).await;
    let second = rig.stand_in.process(0).sent.lock().unwrap()[1].clone();
    assert!(second.contains("and the tabs") && !second.contains("He is aiming for"), "after his first: {second}");

    let started = Arc::clone(&rig.fleet).start_mod_session(spoken(None, Some("a quieter theme"))).await.expect("started");
    Arc::clone(&rig.fleet).send_session_message_aiming(said_by_him(&started.session_id, "and the tabs"), Some(ModAim::Layout)).await.expect("taken");
    eventually(|| async { rig.stand_in.starts().len() == 2 && rig.stand_in.process(1).sent.lock().unwrap().len() == 2 }).await;
    let turns = rig.stand_in.process(1).sent.lock().unwrap().clone();
    assert!(turns.iter().all(|turn| !turn.contains("He is aiming for")), "a Job started on his words: {turns:#?}");

    let plain = serving("armada").await;
    let job = plain.start().await;
    Arc::clone(&plain.fleet).send_session_message_aiming(said_by_him(&job, "look"), Some(ModAim::Theme)).await.expect("taken");
    let turn = turn_of(&plain).await;
    assert!(turn.contains("look") && !turn.contains("He is aiming for") && !turn.contains("This is a Mod Job"), "{turn}");
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
