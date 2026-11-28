//! The claim, written before the code: **his words in All reach one project's Operator, chosen by signals Fleet can read and
//! never by a model, and where none is sure Fleet asks him which and keeps his words until he picks.** A card, a line of the
//! feed and an item to review name the Operator they belong to. `crates/ipc/operations/talk_to_operator.toml`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorFeedKind};
use ipc::operator_talk::TalkToOperator;
use ipc::{CardOption, OperatorCardKind, OperatorSettingsSet, OperatorTag, RaiseCard, SessionId};

use super::operator_cards::said;
use super::operator_projects::project_named;
use super::operator_projects_rig::{caller_on, heard_by, operator_of, projects_rig_placed, set_for};
use super::operator_session::switched;
use super::session_host::eventually;
use crate::operator::routing::{route, RouteInput, RouteProject, RouteRepository, Routed};
use crate::operator::FLEET_CARD;

fn project(id: &str, name: &str) -> RouteProject {
    RouteProject { id: id.into(), name: name.into(), repositories: Vec::new(), features: Vec::new(), on: true, running: true }
}

fn holding(mut one: RouteProject, repository: &str, root: &str) -> RouteProject {
    one.repositories.push(RouteRepository { name: repository.into(), root: PathBuf::from(root) });
    one
}

/// Alpha and Beta, both running, `talk` going to neither by anything but what a test adds.
fn input<'a>(talk: &'a TalkToOperator, projects: Vec<RouteProject>) -> RouteInput<'a> {
    RouteInput { talk, card_project: None, looking_at_project: None, projects, last_talked: None }
}

fn nowhere(_: &Path) -> bool {
    false
}

fn to(id: &str) -> Routed {
    Routed::To(id.into())
}

#[test]
fn a_card_he_talks_about_decides_before_anything_else() {
    let mut talk = said("What about Beta?", Some("turn:1"));
    talk.project = Some("b".into());
    let mut with = input(&talk, vec![project("a", "Alpha"), project("b", "Beta")]);
    with.card_project = Some("a".into());
    assert_eq!(route(&with, &nowhere), to("a"), "the card's project, though his words name Beta and the screen is Beta's");
}

#[test]
fn the_screen_he_is_on_decides_next() {
    let mut talk = said("Anyone?", None);
    talk.project = Some("b".into());
    let mut with = input(&talk, vec![project("a", "Alpha"), project("b", "Beta")]);
    with.looking_at_project = Some("a".into());
    assert_eq!(route(&with, &nowhere), to("b"));
}

#[test]
fn the_job_he_has_open_decides_where_the_screen_names_none() {
    let mut talk = said("Alpha, I mean Beta", None);
    talk.looking_at = Some(SessionId::carried("job-1"));
    let mut with = input(&talk, vec![project("a", "Alpha"), project("b", "Beta")]);
    with.looking_at_project = Some("a".into());
    assert_eq!(route(&with, &nowhere), to("a"), "before his words are read");
}

#[test]
fn one_project_or_one_repository_named_whole_and_whatever_its_case_decides() {
    let talk = said("Look at beta again", None);
    assert_eq!(route(&input(&talk, vec![project("a", "Alpha"), project("b", "Beta")]), &nowhere), to("b"), "a project's name");

    let talk = said("The reader in Shop is slow", None);
    let projects = vec![holding(project("a", "Alpha"), "shop", "/w/shop"), holding(project("b", "Beta"), "garden", "/w/garden")];
    assert_eq!(route(&input(&talk, projects.clone()), &nowhere), to("a"), "a repository's name");

    let talk = said("Betamax is not Beta", None);
    assert_eq!(route(&input(&talk, vec![project("a", "Alpha"), project("b", "Beta")]), &nowhere), to("b"), "a whole word only");
    let talk = said("Betamax only", None);
    assert!(matches!(route(&input(&talk, vec![project("a", "Alpha"), project("b", "Beta")]), &nowhere), Routed::Unsure(_)), "Betamax is not Beta");

    let talk = said("Alpha and Beta both", None);
    assert!(matches!(route(&input(&talk, vec![project("a", "Alpha"), project("b", "Beta")]), &nowhere), Routed::Unsure(_)), "two named is not one");
}

#[test]
fn a_hash_feature_in_one_projects_library_decides() {
    let mut a = project("a", "Alpha");
    a.features = vec!["reader".into()];
    let mut b = project("b", "Beta");
    b.features = vec!["voting".into()];
    let talk = said("Why is #Voting slow?", None);
    assert_eq!(route(&input(&talk, vec![a.clone(), b.clone()]), &nowhere), to("b"));

    b.features.push("reader".into());
    let talk = said("Why is #reader slow?", None);
    assert!(matches!(route(&input(&talk, vec![a, b]), &nowhere), Routed::Unsure(_)), "a slug two projects have names neither");
}

#[test]
fn a_path_that_exists_under_one_projects_repository_decides() {
    let projects = vec![holding(project("a", "Alpha"), "r1", "/w/one"), holding(project("b", "Beta"), "r2", "/w/two")];
    let talk = said("Read crates/pane/lib.rs please", None);
    let only_in_two = |path: &Path| path == Path::new("/w/two/crates/pane/lib.rs");
    assert_eq!(route(&input(&talk, projects.clone()), &only_in_two), to("b"), "a relative path");

    let talk = said("Read /w/one/src/main.rs please", None);
    let there = |path: &Path| path == Path::new("/w/one/src/main.rs");
    assert_eq!(route(&input(&talk, projects.clone()), &there), to("a"), "an absolute one");

    let talk = said("Read crates/pane/lib.rs please", None);
    let in_both = |path: &Path| path.ends_with("crates/pane/lib.rs");
    assert!(matches!(route(&input(&talk, projects), &in_both), Routed::Unsure(_)), "under two is under neither");
}

#[test]
fn the_one_operator_that_runs_decides_last() {
    let talk = said("Anyone?", None);
    let mut idle = project("b", "Beta");
    idle.running = false;
    assert_eq!(route(&input(&talk, vec![project("a", "Alpha"), idle]), &nowhere), to("a"));
}

#[test]
fn where_none_decides_the_four_likeliest_are_asked_by_hits_then_the_one_he_last_talked_to() {
    let projects: Vec<RouteProject> = ["a", "b", "c", "d", "e"].iter().map(|id| project(id, &format!("P{id}"))).collect();
    let talk = said("Anyone?", None);
    let mut with = input(&talk, projects.clone());
    assert_eq!(route(&with, &nowhere), Routed::Unsure(vec!["a".into(), "b".into(), "c".into(), "d".into()]), "four, as they stand");

    with.last_talked = Some("d".into());
    assert_eq!(route(&with, &nowhere), Routed::Unsure(vec!["d".into(), "a".into(), "b".into(), "c".into()]), "the last talked to first among equals");

    // Two projects hold a repository he names and one of them also holds a path: it has two hits.
    let mut shared = projects;
    shared[3] = holding(shared[3].clone(), "shop", "/w/shop");
    shared[4] = holding(shared[4].clone(), "shop", "/w/shop");
    shared[4].features = vec!["reader".into()];
    let talk = said("Shop and #reader", None);
    let mut with = input(&talk, shared);
    with.last_talked = Some("a".into());
    assert_eq!(route(&with, &nowhere), to("e"), "the one feature tag names decides before the shared repository is asked");

    let talk = said("Shop only", None);
    let mut with = input(&talk, with.projects);
    with.last_talked = Some("a".into());
    assert_eq!(route(&with, &nowhere), Routed::Unsure(vec!["d".into(), "e".into(), "a".into(), "b".into()]), "hits first, then the last talked to");

    let mut off = project("o", "Off");
    off.on = false;
    off.running = false;
    let talk = said("Anyone?", None);
    assert_eq!(route(&input(&talk, vec![project("a", "Alpha"), project("b", "Beta"), off]), &nowhere), Routed::Unsure(vec!["a".into(), "b".into()]), "a project whose Operator is off is not asked");
}

/// Alpha and Beta, each with an Operator running.
async fn two_running() -> (super::session_host::Rig, String, String, String, String, Arc<super::peer::Placing>, super::tmp::TempDir) {
    let (rig, second, home, peers) = projects_rig_placed();
    let (a, b) = (project_named(&rig, "Alpha", &rig.manifest).await, project_named(&rig, "Beta", &second).await);
    switched(&rig, true).await;
    rig.fleet.operator_session_kept().await;
    let (for_a, for_b) = (operator_of(&rig, &a).await.expect("A's"), operator_of(&rig, &b).await.expect("B's"));
    eventually(|| async { rig.stand_in.process_of(&for_a).is_some() && rig.stand_in.process_of(&for_b).is_some() }).await;
    (rig, a, b, for_a, for_b, peers, home)
}

async fn fleet_cards(rig: &super::session_host::Rig) -> Vec<store::KeptTurn> {
    rig.fleet.store().lock().await.operator_turn_open().expect("read").into_iter().filter(|row| row.item.starts_with(FLEET_CARD)).collect()
}

#[tokio::test]
async fn two_equal_candidates_raise_the_asking_card_and_his_pick_delivers_his_words_to_that_operator_and_no_other() {
    let (rig, _a, _b, for_a, for_b, _peers, _home) = two_running().await;

    Arc::clone(&rig.fleet).operator_talked(said("Anyone about the reader?", None)).await.expect("kept");
    assert_eq!(heard_by(&rig, &for_a, "Anyone about the reader?") + heard_by(&rig, &for_b, "Anyone about the reader?"), 0, "neither hears it yet");
    let held = rig.fleet.store().lock().await.operator_held_sends().expect("read");
    assert_eq!(held.len(), 1, "his words are kept");
    assert!(!held[0].1.contains("\"project\""), "with no project: {}", held[0].1);

    let cards = fleet_cards(&rig).await;
    assert_eq!(cards.len(), 1, "one card to him");
    let item = ipc::operator_page::item_read(&cards[0].body).expect("an item");
    assert_eq!(item.text, "Which project is this for?");
    let options: Vec<(&str, Option<&str>)> = item.options.iter().map(|one| (one.label.as_str(), one.description.as_deref())).collect();
    let cost = Some("Its Operator hears this and nothing of the others");
    assert_eq!(options, [("Alpha", cost), ("Beta", cost)], "both candidates");

    let press = OperatorAnswerPress { id: format!("turn:{}", cards[0].n), answer: OperatorAnswer::Yes, project: None, then: None, option: Some("Beta".into()) };
    rig.fleet.operator_answered(press).await.expect("pressed");
    eventually(|| async { heard_by(&rig, &for_b, "Anyone about the reader?") >= 1 }).await;
    assert_eq!(heard_by(&rig, &for_a, "Anyone about the reader?"), 0, "Alpha's Operator never hears it");
    assert_eq!(heard_by(&rig, &for_b, "Anyone about the reader?"), 1, "Beta's once");
    assert!(rig.fleet.store().lock().await.operator_held_sends().expect("read").is_empty(), "told, so no longer kept");
    assert!(fleet_cards(&rig).await.is_empty(), "the card is settled");
}

#[tokio::test]
async fn a_send_in_all_with_one_project_on_goes_straight_to_its_operator_with_no_card() {
    let (rig, a, b, for_a, _for_b, _peers, _home) = two_running().await;
    set_for(&rig, &b, OperatorSettingsSet { on: Some(false), ..Default::default() }).await;
    rig.fleet.operator_session_kept().await;
    assert!(operator_of(&rig, &b).await.is_none(), "Beta's Operator is off");

    Arc::clone(&rig.fleet).operator_talked(said("Anyone?", None)).await.expect("said");
    eventually(|| async { heard_by(&rig, &for_a, "Anyone?") >= 1 }).await;
    assert!(fleet_cards(&rig).await.is_empty(), "nothing is asked");
    assert!(rig.fleet.store().lock().await.operator_held_sends().expect("read").is_empty());
    assert_eq!(a, operator_project_of(&rig, &for_a).await);
}

async fn operator_project_of(rig: &super::session_host::Rig, session: &str) -> String {
    rig.fleet.operator_project_of(session).await.expect("an Operator's project")
}

#[tokio::test]
async fn the_screen_he_is_on_and_a_project_he_names_reach_that_operator_without_a_card() {
    let (rig, _a, b, for_a, for_b, _peers, _home) = two_running().await;
    let mut talk = said("Named by the screen", None);
    talk.project = Some(b.clone());
    Arc::clone(&rig.fleet).operator_talked(talk).await.expect("said");
    Arc::clone(&rig.fleet).operator_talked(said("Look at Beta again", None)).await.expect("said");
    eventually(|| async { heard_by(&rig, &for_b, "Named by the screen") >= 1 && heard_by(&rig, &for_b, "Look at Beta again") >= 1 }).await;
    assert_eq!(heard_by(&rig, &for_a, "Beta"), 0);
    assert!(fleet_cards(&rig).await.is_empty());
}

#[tokio::test]
async fn a_card_a_feed_line_and_an_item_to_review_carry_the_name_and_accent_of_their_projects_operator() {
    let (rig, a, b, for_a, _for_b, peers, _home) = two_running().await;
    set_for(&rig, &a, OperatorSettingsSet { name: Some("Kit".into()), accent: Some("#aabbcc".into()), ..Default::default() }).await;
    let tag = Some(OperatorTag { project: a.clone(), name: "Kit".into(), accent: Some("#aabbcc".into()) });

    let caller = caller_on(&rig, &peers, &for_a, 52_300);
    let card = RaiseCard {
        kind: OperatorCardKind::Deal,
        title: "Start the pane Job?".into(),
        read: "Nothing else touches the pane.".into(),
        options: vec![CardOption { label: "Start it".into(), cost: "A slot".into() }, CardOption { label: "Wait".into(), cost: "Nothing moves".into() }],
        sessions: Vec::new(),
        repository: None,
        talk_of: None,
    };
    Arc::clone(&rig.fleet).card_raised(Some(caller), card).await.expect("raised");
    rig.fleet.feed_kept(Some(&a), OperatorFeedKind::Settled, "Start the pane Job?", None, Some("Started it")).await;
    rig.fleet.feed_kept(Some(&b), OperatorFeedKind::Told, "A report", None, None).await;

    let page = rig.fleet.operator_page_wire().await.expect("read");
    assert_eq!(page.turn.iter().find(|one| one.item.text == "Start the pane Job?").map(|one| one.operator.clone()), Some(tag.clone()), "on the card");
    let feed = |title: &str| page.feed.iter().find(|one| one.title == title).map(|one| one.operator.clone());
    assert_eq!(feed("Start the pane Job?"), Some(tag), "on the line");
    assert_eq!(feed("A report"), Some(Some(OperatorTag { project: b, name: "Operator".into(), accent: None })), "the default name where none is set, and no accent");
}
