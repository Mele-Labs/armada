//! Keeping a teaching: the note becomes a Logbook entry with the Session it was taught in as its
//! evidence, each part that changes something becomes a mod that is on, and a live Session is told
//! the note, queued behind a turn it is in the middle of and never refused.

use std::sync::Arc;

use api::{HostedSessions, Logbook, Mods, Refusal, Teaching};
use ipc::{
    Does, FleetRule, LogbookKind, LogbookSection, ModKind, RuleScope, SaveTeaching, SessionId, TeachingLayout,
    TeachingNote, TeachingProposal, TeachingReach, TeachingSource, TeachingSourceKind, TeachingWords, TweakChange,
};

use super::session_host::{eventually, rig_placing, Rig};

const KEY_PATTERN: &str = r"[A-Z][A-Z0-9]+-\d+";

fn words(example: &str, address: Option<&str>) -> TeachingWords {
    TeachingWords {
        name: "Jira key".to_string(),
        pattern: KEY_PATTERN.to_string(),
        example: example.to_string(),
        address: address.map(str::to_string),
    }
}

fn note(text: &str) -> TeachingNote {
    TeachingNote { section: LogbookSection::Repo, kind: LogbookKind::Convention, text: text.to_string() }
}

fn saving(proposal: TeachingProposal, rig: Option<&Rig>) -> SaveTeaching {
    SaveTeaching {
        proposal,
        reach: if rig.is_some() { TeachingReach::Repository } else { TeachingReach::Everywhere },
        repository: rig.map(|rig| rig.root.clone()),
        session_id: None,
        cite: None,
        source: None,
    }
}

fn from_words(rig: &Rig, text: &str) -> SaveTeaching {
    SaveTeaching {
        session_id: Some("s1".to_string()),
        cite: Some("msg-3".to_string()),
        source: Some(TeachingSource { kind: TeachingSourceKind::Words, text: text.to_string(), ids: Vec::new() }),
        ..saving(TeachingProposal::default(), Some(rig))
    }
}

fn code(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.clone())
}

async fn mods_named(rig: &Rig) -> Vec<(String, Option<ModKind>, bool, Option<String>)> {
    let list = rig.fleet.list_mods().await.expect("listed");
    list.mods.into_iter().map(|one| (one.name, one.kind, one.valid, one.taught_on)).collect()
}

async fn rules(rig: &Rig, repository: Option<&str>) -> Vec<(String, String)> {
    let listed = Arc::clone(&rig.fleet).list_word_rules(repository.map(str::to_string), None).await.expect("listed");
    listed.rules.into_iter().map(|one| (one.mod_name, one.address)).collect()
}

#[tokio::test]
async fn a_teaching_from_a_session_makes_an_entry_with_its_source_and_a_words_mod_that_records_the_entry() {
    let rig = rig_placing(None);
    let mut save = from_words(&rig, "ARM-123");
    save.proposal.note = Some(note("ARM keys are Jira tickets."));
    save.proposal.words = Some(words("ARM-123", Some("https://acme.example.net/browse/{match}")));
    let saved = Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");

    let entry_id = saved.entry.expect("an entry");
    assert_eq!(saved.mods, ["jira-key"]);
    let logbook = rig.fleet.get_logbook(Some(rig.root.clone()), None, None).await.expect("read");
    let entry = logbook.entries.iter().find(|one| one.id == entry_id).expect("the entry");
    assert_eq!(entry.text, "ARM keys are Jira tickets.");
    assert_eq!((entry.evidence[0].job_id.as_str(), entry.evidence[0].cite.as_str()), ("s1", "msg-3"));

    assert_eq!(mods_named(&rig).await, [("jira-key".to_string(), Some(ModKind::Words), true, Some("ARM-123".to_string()))]);
    let toml = std::fs::read_to_string(rig.fleet.mods_dir().join("jira-key/mod.toml")).expect("mod.toml");
    assert!(toml.contains(&format!("entry = \"{entry_id}\"")) && toml.contains("taught_on = \"ARM-123\""), "{toml}");
    let manifest = rig.fleet.manifest_id_at(&rig.root).expect("manifest");
    let file = std::fs::read_to_string(rig.fleet.mods_dir().join("jira-key/words.toml")).expect("words.toml");
    assert!(file.contains(&format!("repository = \"{manifest}\"")) && file.contains("found = \"person\""), "{file}");
    assert_eq!(rules(&rig, Some(&rig.root)).await, [("jira-key".to_string(), "https://acme.example.net/browse/{match}".to_string())]);
    assert!(rules(&rig, None).await.is_empty(), "an address for one repository is not one for every repository");
}

#[tokio::test]
async fn a_pattern_taught_again_gains_an_address_and_is_not_made_twice() {
    let rig = rig_placing(None);
    let mut first = saving(TeachingProposal::default(), None);
    first.proposal.words = Some(words("ARM-123", Some("https://any.example.net/{match}")));
    Arc::clone(&rig.fleet).save_teaching(first).await.expect("saved");
    let mut second = saving(TeachingProposal::default(), Some(&rig));
    second.proposal.words = Some(words("ARM-9", Some("https://acme.example.net/browse/{match}")));
    let saved = Arc::clone(&rig.fleet).save_teaching(second).await.expect("saved");

    assert_eq!(saved.mods, ["jira-key"], "the same mod");
    assert_eq!(mods_named(&rig).await.len(), 1);
    assert_eq!(rules(&rig, Some(&rig.root)).await[0].1, "https://acme.example.net/browse/{match}", "the repository's own address wins");
    assert_eq!(rules(&rig, None).await[0].1, "https://any.example.net/{match}", "and every repository has the one for all");
    let file = std::fs::read_to_string(rig.fleet.mods_dir().join("jira-key/words.toml")).expect("words.toml");
    assert_eq!(file.matches("[[address]]").count(), 2, "{file}");
}

#[tokio::test]
async fn a_teaching_on_bridge_makes_mods_alone_that_say_what_they_were_taught_on() {
    let rig = rig_placing(None);
    let mut save = saving(TeachingProposal::default(), None);
    save.source = Some(TeachingSource { kind: TeachingSourceKind::Panel, text: "The Cockpit's merge line panel".into(), ids: vec!["merge-line".into()] });
    save.proposal.tweak = vec![TweakChange {
        token: "--bg-raised".into(),
        was: "ignored".into(),
        now: "#1F1A15".into(),
        used_in: Vec::new(),
        note: None,
    }];
    save.proposal.layout = Some(TeachingLayout {
        name: "Tidy".into(),
        layout: r#"{"version":1,"dashboard.panels":{"hidden":["merge-line"]}}"#.into(),
    });
    let saved = Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");

    assert_eq!(saved.entry, None, "nothing here is for an agent");
    assert_eq!(saved.mods, ["tweak-bg-raised", "tidy"]);
    let mods = mods_named(&rig).await;
    assert!(mods.iter().all(|(_, _, valid, on)| *valid && on.as_deref() == Some("The Cockpit's merge line panel")), "{mods:?}");
    let css = std::fs::read_to_string(rig.fleet.mods_dir().join("tweak-bg-raised/tweak.css")).expect("tweak.css");
    assert_eq!(css, ":root {\n  --bg-raised: #1F1A15;\n}\n");
    let toml = std::fs::read_to_string(rig.fleet.mods_dir().join("tidy/mod.toml")).expect("mod.toml");
    assert!(!toml.contains("entry"), "{toml}");
    let checked = rig.fleet.validate_mod("tweak-bg-raised".into()).await.expect("checked");
    assert_eq!(checked.css.as_deref(), Some(css.as_str()), "Bridge is handed the tweak as css");

    let again = saving(TeachingProposal { tweak: vec![TweakChange { token: "--bg-raised".into(), was: String::new(), now: "#222".into(), used_in: Vec::new(), note: None }], ..TeachingProposal::default() }, None);
    assert_eq!(Arc::clone(&rig.fleet).save_teaching(again).await.expect("saved").mods, ["tweak-bg-raised-2"], "a taken name is not reused");
}

#[tokio::test]
async fn a_fleet_rule_is_a_mod_and_the_nearest_place_wins_and_a_switched_off_mod_says_nothing() {
    let rig = rig_placing(None);
    let manifest = rig.fleet.manifest_id_at(&rig.root).expect("manifest");
    let rule = |does, scope| FleetRule { notice: "behind_base".into(), does, scope };
    for one in [rule(Does::Batch, RuleScope::Everywhere), rule(Does::Quiet, RuleScope::Repo(manifest.clone())), rule(Does::Call, RuleScope::Job("s9".into()))] {
        let save = saving(TeachingProposal { fleet: Some(one), ..TeachingProposal::default() }, None);
        Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");
    }
    let asked = |session: &'static str, repository: &str| {
        let (fleet, repository) = (Arc::clone(&rig.fleet), repository.to_string());
        async move { fleet.fleet_rule_for("behind_base", session, &repository).await }
    };
    assert_eq!(asked("s9", &manifest).await, Some(Does::Call));
    assert_eq!(asked("s1", &manifest).await, Some(Does::Quiet));
    assert_eq!(asked("s1", "elsewhere").await, Some(Does::Batch));
    assert_eq!(rig.fleet.fleet_rule_for("another_notice", "s1", &manifest).await, None);

    rig.fleet.set_mod_enabled(ipc::SetModEnabled { name: "fleet-behind-base-2".into(), enabled: false }).await.expect("off");
    assert_eq!(asked("s1", &manifest).await, Some(Does::Batch), "the repository's rule is off");
}

#[tokio::test]
async fn a_switched_off_words_mod_links_nothing() {
    let rig = rig_placing(None);
    let mut save = saving(TeachingProposal::default(), None);
    save.proposal.words = Some(words("ARM-123", Some("https://any.example.net/{match}")));
    Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");
    assert_eq!(rules(&rig, None).await.len(), 1);
    rig.fleet.set_mod_enabled(ipc::SetModEnabled { name: "jira-key".into(), enabled: false }).await.expect("off");
    assert!(rules(&rig, None).await.is_empty());
}

#[tokio::test]
async fn a_part_that_cannot_work_is_refused_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let bad = |change: &dyn Fn(&mut TeachingProposal)| {
        let mut proposal = TeachingProposal { note: Some(note("A fine note.")), ..TeachingProposal::default() };
        change(&mut proposal);
        saving(proposal, Some(&rig))
    };
    let cases: Vec<(SaveTeaching, &str)> = vec![
        (bad(&|p| p.words = Some(words("BLOG-1", None)).map(|mut w| { w.pattern = r"STORE-\d+".into(); w })), "fleet.teaching_words_refused"),
        (bad(&|p| p.words = Some(words("ARM-1", Some("ftp://x.example/{match}")))), "fleet.teaching_words_refused"),
        (bad(&|p| p.note = Some(TeachingNote { section: LogbookSection::Habits, kind: LogbookKind::Habit, text: "x".into() })), "fleet.teaching_note_refused"),
        (bad(&|p| p.fleet = Some(FleetRule { notice: "made_up".into(), does: Does::Quiet, scope: RuleScope::Everywhere })), "fleet.teaching_rule_refused"),
        (bad(&|p| p.tweak = vec![TweakChange { token: "--accent".into(), was: String::new(), now: "url(x)".into(), used_in: vec![], note: None }]), "fleet.teaching_tweak_refused"),
        (bad(&|p| p.layout = Some(TeachingLayout { name: "x".into(), layout: "not json".into() })), "fleet.teaching_layout_refused"),
        (saving(TeachingProposal::default(), Some(&rig)), "fleet.teaching_input_refused"),
    ];
    for (save, wanted) in cases {
        let refused = Arc::clone(&rig.fleet).save_teaching(save).await.expect_err("refused");
        assert_eq!(code(refused), (422, wanted.to_string()));
    }
    let mut unplaced = saving(TeachingProposal { note: Some(note("Fine.")), ..TeachingProposal::default() }, None);
    unplaced.reach = TeachingReach::Repository;
    assert_eq!(code(Arc::clone(&rig.fleet).save_teaching(unplaced).await.expect_err("no repository")), (422, "fleet.teaching_input_refused".to_string()));
    let mut elsewhere = saving(TeachingProposal { note: Some(note("Fine.")), ..TeachingProposal::default() }, Some(&rig));
    elsewhere.repository = Some("/nowhere".into());
    assert_eq!(code(Arc::clone(&rig.fleet).save_teaching(elsewhere).await.expect_err("not served")).0, 404);

    assert!(mods_named(&rig).await.is_empty(), "no mod was written");
    let logbook = rig.fleet.get_logbook(Some(rig.root.clone()), None, None).await.expect("read");
    assert!(logbook.entries.iter().all(|one| one.text != "A fine note."), "and the note that came with them was not kept");
}

fn told(session: &SessionId, text: &str) -> SaveTeaching {
    let mut save = saving(TeachingProposal { note: Some(TeachingNote { section: LogbookSection::Habits, kind: LogbookKind::Habit, text: text.into() }), ..TeachingProposal::default() }, None);
    save.session_id = Some(session.as_str().to_string());
    save
}

fn lines_to(rig: &Rig) -> Vec<String> {
    rig.stand_in.process(0).sent.lock().unwrap().clone()
}

#[tokio::test]
async fn an_idle_session_is_told_the_note_in_fleets_voice() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    let before = lines_to(&rig).len();

    Arc::clone(&rig.fleet).save_teaching(told(&id, "Propose ideas with a sketch.")).await.expect("saved");

    let lines = lines_to(&rig);
    assert_eq!(lines.len(), before + 1, "one turn was started");
    assert!(lines.last().unwrap().contains("From now on: Propose ideas with a sketch."));
}

#[tokio::test]
async fn a_session_in_the_middle_of_a_turn_has_the_note_queued_and_it_is_not_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    // The turn began once its first line was heard; until then it is still counted as queued.
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().queued == 0 }).await;
    assert_eq!(rig.turn(&id).await, ipc::SessionTurn::Working { woken_by: None }, "the turn has not ended");
    let before = lines_to(&rig).len();

    let saved = Arc::clone(&rig.fleet).save_teaching(told(&id, "Use the focused test command.")).await;
    assert!(saved.is_ok(), "{:?}", saved.err().map(code));

    let lines = lines_to(&rig);
    assert_eq!(lines.len(), before + 1, "the line was handed to the process behind the running turn");
    assert!(lines.last().unwrap().contains("From now on: Use the focused test command."));
    assert_eq!(rig.fleet.hosts().of(id.as_str()).state().queued, 1, "counted as a queued turn, not dropped");
}

#[tokio::test]
async fn a_session_that_cannot_be_told_does_not_fail_the_save() {
    let rig = rig_placing(None);
    let gone = SessionId::carried("01NOSUCHSESSIONAAAAAAAAAA");
    let saved = Arc::clone(&rig.fleet).save_teaching(told(&gone, "Whatever.")).await.expect("saved all the same");
    assert!(saved.entry.is_some());

    let ended = rig.start().await;
    Arc::clone(&rig.fleet).close_session(ipc::CloseSession { session_id: ended.clone() }).await.expect("closed");
    assert!(Arc::clone(&rig.fleet).save_teaching(told(&ended, "Whatever else.")).await.is_ok());
}

#[tokio::test]
async fn a_session_is_told_nothing_when_a_teaching_has_no_note() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    let before = lines_to(&rig).len();
    let mut save = saving(TeachingProposal::default(), None);
    save.proposal.words = Some(words("ARM-1", None));
    save.session_id = Some(id.as_str().to_string());
    Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");
    assert_eq!(lines_to(&rig).len(), before, "there was nothing to say");
}
