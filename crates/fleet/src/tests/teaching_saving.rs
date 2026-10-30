//! Keeping a teaching: the note becomes a Logbook entry with the Session it was taught in as its
//! evidence, each part that changes something becomes a mod that is on, and a live Session is told
//! the note, queued behind a turn it is in the middle of and never refused.

use std::sync::Arc;

use api::{HostedSessions, Logbook, Mods, Refusal, Teaching};
use ipc::{
    Does, FleetRule, LifecycleRule, LogbookKind, LogbookSection, ModKind, PieceScope, ProposedRule, RuleAct, RuleAmong, RuleDoing, RuleOnFailure,
    RuleScope, RuleTarget, RuleWhen, SaveTeaching, SessionId, TeachingLayout, TeachingNote, TeachingProposal, TeachingReach, TeachingSource,
    TeachingSourceKind, TeachingWords, TweakChange,
};

use super::session_host::{eventually, rig_placing, Rig};

const KEY_PATTERN: &str = r"[A-Z][A-Z0-9]+-\d+";

fn words(example: &str, address: Option<&str>) -> TeachingWords {
    TeachingWords {
        name: "Jira key".to_string(),
        pattern: KEY_PATTERN.to_string(),
        example: example.to_string(),
        address: address.map(|template| ipc::ProposedAddress { template: template.to_string(), found: ipc::AddressFound::Person }),
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
    let rule = |does, scope| FleetRule { notice: "behind_base".into(), does, scope, matches: None, when: Vec::new() };
    for one in [rule(Does::Batch, RuleScope::Everywhere), rule(Does::Quiet, RuleScope::Repo(manifest.clone())), rule(Does::Call, RuleScope::Job("s9".into()))] {
        let save = saving(TeachingProposal { fleet: Some(ipc::ProposedFleet { rule: one, untestable: Vec::new() }), ..TeachingProposal::default() }, None);
        Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");
    }
    let asked = |session: &'static str, repository: &str| {
        let (fleet, repository) = (Arc::clone(&rig.fleet), repository.to_string());
        async move { fleet.fleet_rule_for("behind_base", "", session, &repository).await.map(|rule| rule.does) }
    };
    assert_eq!(asked("s9", &manifest).await, Some(Does::Call));
    assert_eq!(asked("s1", &manifest).await, Some(Does::Quiet));
    assert_eq!(asked("s1", "elsewhere").await, Some(Does::Batch));
    assert_eq!(rig.fleet.fleet_rule_for("another_notice", "", "s1", &manifest).await, None);

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
        (bad(&|p| p.fleet = Some(ipc::ProposedFleet { rule: FleetRule { notice: "made_up".into(), does: Does::Quiet, scope: RuleScope::Everywhere, matches: None, when: Vec::new() }, untestable: Vec::new() })), "fleet.teaching_rule_refused"),
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

#[tokio::test]
async fn forgetting_the_entry_removes_the_mods_it_made_and_leaves_every_other_mod() {
    let rig = rig_placing(None);
    let mut save = from_words(&rig, "ARM-123");
    save.proposal.note = Some(note("ARM keys are Jira tickets."));
    save.proposal.words = Some(words("ARM-123", Some("https://acme.example.net/browse/{match}")));
    save.proposal.fleet = Some(ipc::ProposedFleet { rule: FleetRule { notice: "behind_base".into(), does: Does::Quiet, scope: RuleScope::Everywhere, matches: None, when: Vec::new() }, untestable: Vec::new() });
    let saved = Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");
    let entry = saved.entry.clone().expect("an entry");
    assert_eq!(saved.mods.len(), 2);

    let mut on_bridge = saving(TeachingProposal::default(), None);
    on_bridge.source = Some(TeachingSource { kind: TeachingSourceKind::Panel, text: "A panel".into(), ids: Vec::new() });
    on_bridge.proposal.tweak = vec![TweakChange { token: "--bg-raised".into(), was: String::new(), now: "#222".into(), used_in: vec![], note: None }];
    Arc::clone(&rig.fleet).save_teaching(on_bridge).await.expect("saved");
    let by_hand = rig.fleet.scaffold_mod(ipc::ScaffoldMod { name: "mine".into(), description: Some("By hand".into()), kind: Some(ModKind::Theme), session_id: None }).await.expect("made");
    assert!(std::path::Path::new(&by_hand.path).is_dir());

    rig.fleet.forget_logbook_entry(entry.clone()).await.expect("forgotten");

    let left: Vec<String> = mods_named(&rig).await.into_iter().map(|one| one.0).collect();
    assert_eq!(left, ["mine", "tweak-bg-raised"], "the words mod and the rule the entry made are gone");
    assert!(rules(&rig, Some(&rig.root)).await.is_empty());
    let second = rig.fleet.forget_logbook_entry(entry).await.expect_err("already forgotten");
    assert_eq!(second.status(), 404);
}

#[test]
fn only_a_real_folder_that_names_the_entry_is_taken() {
    let dir = super::tmp::TempDir::new();
    let make = |name: &str, entry: &str| {
        let mod_dir = dir.path().join(name);
        std::fs::create_dir_all(&mod_dir).expect("folder");
        std::fs::write(mod_dir.join("mod.toml"), format!("name = \"{name}\"\nkind = \"theme\"\nentry = \"{entry}\"\n")).expect("mod.toml");
    };
    make("a", "entry-1");
    make("b", "entry-2");
    make("c", "entry-1");
    std::fs::write(dir.path().join("a-file"), "entry = \"entry-1\"").expect("file");
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.path().join("b"), dir.path().join("linked")).expect("link");

    assert_eq!(crate::teaching::forget_effects(dir.path(), "entry-1"), ["a", "c"]);
    assert!(dir.path().join("b").is_dir() && dir.path().join("a-file").is_file());
    assert!(crate::teaching::forget_effects(dir.path(), "entry-1").is_empty(), "a second time takes nothing");
    assert!(crate::teaching::forget_effects(&dir.path().join("nowhere"), "entry-1").is_empty());
}

#[tokio::test]
async fn a_rule_with_matches_and_conditions_is_written_as_the_file_reads_it_and_untestable_is_not_kept() {
    let rig = rig_placing(None);
    let rule = FleetRule {
        notice: "pr_checks_failed".into(),
        does: Does::Batch,
        scope: RuleScope::Everywhere,
        matches: Some("lint".into()),
        when: vec![ipc::Instead { condition: ipc::Condition::BlocksMerge, does: Does::Tell }],
    };
    let fleet = ipc::ProposedFleet { rule: rule.clone(), untestable: vec!["when it is a Friday".into()] };
    let mut save = saving(TeachingProposal { fleet: Some(fleet), ..TeachingProposal::default() }, None);
    save.source = Some(ipc::TeachingSource { kind: ipc::TeachingSourceKind::Notice, text: "#7 failed its checks: lint".into(), ids: vec!["pr_checks_failed".into()] });
    let saved = Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");

    let file = std::fs::read_to_string(rig.fleet.mods_dir().join(&saved.mods[0]).join("fleet.toml")).expect("written");
    assert!(!file.contains("untestable") && file.contains("blocks_merge") && file.contains("matches = \"lint\""), "{file}");
    assert_eq!(crate::mods::fleet_rules::read(&file).expect("reads"), ipc::FleetFile { rule: vec![rule] });
    assert_eq!(crate::mods::fleet_rules::problems(&file), Vec::<String>::new());
}

#[tokio::test]
async fn a_rule_whose_pattern_cannot_work_or_finds_nothing_in_the_notice_it_was_taught_on_is_refused() {
    let rig = rig_placing(None);
    let ruled = |matches: &str, text: &str| {
        let rule = FleetRule { notice: "pr_checks_failed".into(), does: Does::Quiet, scope: RuleScope::Everywhere, matches: Some(matches.into()), when: Vec::new() };
        let mut save = saving(TeachingProposal { fleet: Some(ipc::ProposedFleet { rule, untestable: Vec::new() }), ..TeachingProposal::default() }, None);
        save.source = Some(ipc::TeachingSource { kind: ipc::TeachingSourceKind::Notice, text: text.into(), ids: vec!["pr_checks_failed".into()] });
        save
    };
    for (pattern, text) in [("(a+)+", "aaa"), ("a*", "aaa"), ("lint", "#7 failed: unit tests")] {
        let refused = Arc::clone(&rig.fleet).save_teaching(ruled(pattern, text)).await.expect_err(pattern);
        assert_eq!(code(refused).1, "fleet.teaching_rule_refused", "{pattern}");
    }
}

/// What the panel hands back for "when this is approved, move its tickets to Done": the rule as a
/// read answers it, with the id and scope the read gave and the kind and autonomy worked out.
fn closes_tickets() -> ProposedRule {
    ProposedRule {
        rule: LifecycleRule {
            id: "close_the_tickets_when_approved".to_string(),
            name: "Close the tickets when approved".to_string(),
            scope: PieceScope::Everywhere,
            when: RuleWhen::Enters { lifecycle: "pr".to_string(), state: "approved".to_string() },
            conditions: Vec::new(),
            does: vec![RuleDoing {
                what: RuleAct::TrackerMove {
                    to: "closed".to_string(),
                    target: RuleTarget { among: RuleAmong::LinkedFrom { lifecycle: "ticket".to_string() }, in_state: None },
                },
                kind: "tracker_write".to_string(),
                autonomy: "asks".to_string(),
            }],
            autonomy: None,
            on_failure: RuleOnFailure { retries: 2, then: "ask_owner".to_string() },
        },
        untestable: vec!["when it is not a Friday".to_string()],
    }
}

fn from_pull_request(rig: &Rig, rule: ProposedRule) -> SaveTeaching {
    SaveTeaching {
        source: Some(TeachingSource { kind: TeachingSourceKind::PullRequest, text: "#812 Add the store page".into(), ids: vec!["812".into()] }),
        ..saving(TeachingProposal { rule: Some(rule), ..TeachingProposal::default() }, Some(rig))
    }
}

fn rule_files(rig: &Rig) -> Vec<std::path::PathBuf> {
    let folder = std::path::Path::new(&rig.fleet.host().kit_home).join("rules");
    let mut files: Vec<_> = std::fs::read_dir(folder).map(|found| found.filter_map(Result::ok).map(|one| one.path()).collect()).unwrap_or_default();
    files.sort();
    files
}

#[tokio::test]
async fn a_rule_is_written_as_a_rule_piece_in_kit_scoped_to_the_repository_and_is_not_a_mod() {
    let rig = rig_placing(None);
    let saved = Arc::clone(&rig.fleet).save_teaching(from_pull_request(&rig, closes_tickets())).await.expect("saved");

    let manifest = rig.fleet.manifest_id_at(&rig.root).expect("manifest");
    let piece = saved.rule.expect("the rule piece");
    assert_eq!((piece.id.as_str(), piece.replaced), ("close_the_tickets_when_approved", false));
    assert_eq!(piece.scope, PieceScope::Within { id: manifest.clone() }, "the reach the person chose, whatever the rule carried");
    assert!(saved.mods.is_empty() && saved.entry.is_none(), "a rule is neither a mod nor a note");
    assert!(mods_named(&rig).await.is_empty());

    let file = std::fs::read_to_string(&piece.file).expect("written");
    assert!(file.contains("rule_id: close_the_tickets_when_approved") && file.contains(&format!("scope: {manifest}")), "{file}");
    assert!(!file.contains("Friday"), "untestable is not kept\n{file}");
    let applying = rig.fleet.rules_applying_to(&core_model::ManifestId::carried(core_model::Ulid::carried(&manifest))).expect("read");
    let rule = applying.files().iter().find(|one| one.rule().id().as_str() == "close_the_tickets_when_approved").expect("a read finds it");
    assert_eq!(rule.rank().source, core_model::PieceSource::Kit, "yours, in the Kit");
}

#[tokio::test]
async fn a_rule_taught_for_every_repository_has_no_scope_in_its_file() {
    let rig = rig_placing(None);
    let mut save = from_pull_request(&rig, closes_tickets());
    save.reach = TeachingReach::Everywhere;
    let piece = Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved").rule.expect("the rule piece");
    assert_eq!(piece.scope, PieceScope::Everywhere);
    let file = std::fs::read_to_string(&piece.file).expect("written");
    assert!(!file.contains("scope:"), "{file}");
}

#[tokio::test]
async fn the_id_is_made_from_the_name_the_person_left_and_a_second_rule_of_it_is_a_second_piece() {
    let rig = rig_placing(None);
    let mut renamed = closes_tickets();
    renamed.rule.name = "Tickets done on approval".to_string();
    let first = Arc::clone(&rig.fleet).save_teaching(from_pull_request(&rig, renamed.clone())).await.expect("saved").rule.expect("piece");
    assert_eq!(first.id, "tickets_done_on_approval", "the editor's rule: lower case words joined by underscores");
    let second = Arc::clone(&rig.fleet).save_teaching(from_pull_request(&rig, renamed)).await.expect("saved").rule.expect("piece");
    assert_eq!((second.id.as_str(), second.replaced), ("tickets_done_on_approval_2", false), "never over the first");
    assert_eq!(rule_files(&rig).len(), 2);
}

#[tokio::test]
async fn a_rule_a_read_of_its_file_would_set_aside_is_refused_and_the_note_with_it_is_not_kept() {
    let rig = rig_placing(None);
    let mut lifecycle_missing = closes_tickets();
    lifecycle_missing.rule.when = RuleWhen::Enters { lifecycle: "release".to_string(), state: "out".to_string() };
    let mut state_missing = closes_tickets();
    state_missing.rule.when = RuleWhen::Enters { lifecycle: "pr".to_string(), state: "aproved".to_string() };
    let mut does_nothing = closes_tickets();
    does_nothing.rule.does.clear();
    let mut unnamed = closes_tickets();
    unnamed.rule.name = "  ".to_string();
    for (why, rule) in [("a lifecycle that does not apply", lifecycle_missing), ("a state it lacks", state_missing), ("no acts", does_nothing), ("no name", unnamed)] {
        let mut save = from_pull_request(&rig, rule);
        save.proposal.note = Some(note("A fine note."));
        let refused = Arc::clone(&rig.fleet).save_teaching(save).await.expect_err(why);
        assert_eq!(code(refused), (422, "fleet.teaching_rule_refused".to_string()), "{why}");
    }
    assert!(rule_files(&rig).is_empty(), "nothing was written");
    let logbook = rig.fleet.get_logbook(Some(rig.root.clone()), None, None).await.expect("read");
    assert!(logbook.entries.iter().all(|one| one.text != "A fine note."), "and the note that came with it was not kept");

    let mut placed = from_pull_request(&rig, closes_tickets());
    placed.repository = None;
    assert_eq!(code(Arc::clone(&rig.fleet).save_teaching(placed).await.expect_err("no repository")), (422, "fleet.teaching_input_refused".to_string()));
}

#[tokio::test]
async fn a_rule_comes_with_the_note_a_ticket_was_taught_and_only_the_note_is_told() {
    let rig = rig_placing(None);
    let mut save = from_pull_request(&rig, closes_tickets());
    save.proposal.note = Some(note("Tickets close when their pull request is approved."));
    let saved = Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");
    assert!(saved.entry.is_some() && saved.rule.is_some() && saved.mods.is_empty());
}
