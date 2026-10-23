//! Where taught words open in a repository nobody wrote an address for: the Manifest's tracker
//! first, a merged pull request's text second, and otherwise one question to the person, once.
//! The search is lazy and off the request, so these wait for it with `eventually`.

use std::sync::Arc;

use adapter_traits::{FromOutside, RecentlyMerged};
use api::{HostedSessions, Teaching};
use ipc::{
    AnswerWaiting, DismissWaiting, SaveTeaching, SessionId, TeachingProposal, TeachingReach,
    TeachingWords, WaitingItem, WaitingSource,
};
use testkit::{FakeTracker, FakeTrackers};

use super::pull_request_panel::a_page;
use super::session_host::{eventually, rig_fitted, Rig};
use crate::teaching::finding::{template_in, Begin, Searched};

const KEY: &str = r"[A-Z][A-Z0-9]+-\d+";

#[test]
fn a_web_address_with_a_match_in_it_becomes_a_template() {
    assert_eq!(
        template_in("Fixes it. See https://acme.example.net/browse/ARM-123.", KEY).as_deref(),
        Some("https://acme.example.net/browse/{match}")
    );
    assert_eq!(
        template_in("[ARM-9](https://t.example/issue?id=ARM-9&view=full)", KEY).as_deref(),
        Some("https://t.example/issue?id={match}&view=full")
    );
    assert_eq!(
        template_in("http://a.example/x then https://b.example/browse/OPS-7/comments", KEY).as_deref(),
        Some("https://b.example/browse/{match}/comments"),
        "the first address that holds a match"
    );
}

#[test]
fn a_match_that_is_not_alone_in_its_place_or_is_in_the_host_is_no_address() {
    for text in [
        "https://ARM-1.example.com/page",
        "https://x.example/fooARM-1",
        "https://x.example/ARM-1x",
        "ftp://x.example/browse/ARM-1",
        "browse/ARM-1 without a scheme",
        "https://forge.example/acme/armada/pull/12",
        "",
    ] {
        assert_eq!(template_in(text, KEY), None, "{text}");
    }
    assert_eq!(template_in("https://x.example/browse/ARM-1", "(unclosed"), None, "a pattern that does not compile finds nothing");
}

#[test]
fn a_search_is_claimed_once_and_a_miss_is_remembered() {
    let searched = Searched::default();
    let key = ("jira-key".to_string(), "armada".to_string());
    assert_eq!(searched.begin(&key), Begin::Search);
    assert_eq!(searched.begin(&key), Begin::Busy);
    searched.ended(&key, false);
    assert_eq!(searched.begin(&key), Begin::Missed);
    assert_eq!(searched.begin(&("jira-key".to_string(), "other".to_string())), Begin::Search, "another repository is its own search");
    let found = ("ticket".to_string(), "armada".to_string());
    assert_eq!(searched.begin(&found), Begin::Search);
    searched.ended(&found, true);
    assert_eq!(searched.begin(&found), Begin::Search, "an address found needs no memory of the search");
}

fn words(address: Option<&str>) -> TeachingWords {
    TeachingWords {
        name: "Jira key".to_string(),
        pattern: KEY.to_string(),
        example: "ARM-123".to_string(),
        address: address.map(str::to_string),
    }
}

fn teaching(rig: &Rig, session: Option<&SessionId>, reach: TeachingReach, address: Option<&str>) -> SaveTeaching {
    SaveTeaching {
        proposal: TeachingProposal { words: Some(words(address)), ..TeachingProposal::default() },
        reach,
        repository: (reach == TeachingReach::Repository).then(|| rig.root.clone()),
        session_id: session.map(|one| one.as_str().to_string()),
        cite: None,
        source: None,
    }
}

fn rig_with_tracker(tracker: Arc<FakeTracker>) -> Rig {
    rig_fitted(|fittings| fittings.trackers = Arc::new(FakeTrackers::empty().tracker(tracker)))
}

async fn rules(rig: &Rig, session: Option<&SessionId>) -> Vec<String> {
    let session = session.map(|one| one.as_str().to_string());
    let listed = Arc::clone(&rig.fleet).list_word_rules(Some(rig.root.clone()), session).await.expect("listed");
    listed.rules.into_iter().map(|one| one.address).collect()
}

fn words_file(rig: &Rig) -> String {
    std::fs::read_to_string(rig.fleet.mods_dir().join("jira-key/words.toml")).expect("words.toml")
}

async fn calls(rig: &Rig, session: &SessionId) -> Vec<WaitingItem> {
    let record = Arc::clone(&rig.fleet).get_session(session.clone()).await.expect("read").session;
    record.waiting_for.into_iter().filter(|one| one.source == WaitingSource::WordsAddress).collect()
}

fn merged(number: u64) -> RecentlyMerged {
    RecentlyMerged {
        number,
        title: FromOutside::verbatim("A change"),
        branch: FromOutside::verbatim("armada/x"),
        url: FromOutside::verbatim("https://forge.invalid/pull/1"),
        author: None,
        merged_at: FromOutside::verbatim("2026-10-20T10:00:00Z"),
        commit: None,
    }
}

#[tokio::test]
async fn the_trackers_site_is_the_first_place_looked_and_the_address_lands_off_the_request() {
    let tracker = Arc::new(FakeTracker::empty());
    tracker.opens_at("https://acme.example.net/browse/{match}");
    let rig = rig_with_tracker(tracker);
    let session = rig.start().await;
    Arc::clone(&rig.fleet).save_teaching(teaching(&rig, Some(&session), TeachingReach::Repository, None)).await.expect("saved");

    eventually(|| async { rules(&rig, None).await == ["https://acme.example.net/browse/{match}"] }).await;
    let file = words_file(&rig);
    assert!(file.contains("found = \"settings\""), "{file}");
    assert!(calls(&rig, &session).await.is_empty(), "nobody is asked where it was found");
}

#[tokio::test]
async fn reading_the_rules_for_a_repository_finds_the_address_a_save_for_every_repository_could_not() {
    let tracker = Arc::new(FakeTracker::empty());
    tracker.opens_at("https://acme.example.net/browse/{match}");
    let rig = rig_with_tracker(tracker);
    let session = rig.start().await;
    // No repository on the save, so there is nothing to look for yet.
    let mut everywhere = teaching(&rig, None, TeachingReach::Everywhere, None);
    everywhere.proposal.words.as_mut().unwrap().address = None;
    Arc::clone(&rig.fleet).save_teaching(everywhere).await.expect("saved");
    assert!(rules(&rig, None).await.is_empty());

    eventually(|| async { rules(&rig, Some(&session)).await.len() == 1 }).await;
    assert!(words_file(&rig).contains("found = \"settings\""));
}

#[tokio::test]
async fn a_merged_pull_requests_text_is_the_second_place_and_the_address_names_the_pull_request() {
    let rig = rig_fitted(|fittings| {
        let file = std::path::Path::new("armada.yml");
        fittings.starting().manifest = config::Manifest::parse(file, "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\n").expect("parses");
    });
    let session = rig.start().await;
    rig.fleet.vcs().main_ci.recently_merged_are(Some(vec![merged(40), merged(41)]));
    let mut quiet = a_page();
    quiet.body = Some("Takes a draft out.".into());
    let mut linking = a_page();
    linking.body = Some("Closes https://tracker.example.org/issues/ARM-77.".into());
    rig.fleet.vcs().now_page_of_pull_request(40, Some(quiet));
    rig.fleet.vcs().now_page_of_pull_request(41, Some(linking));
    Arc::clone(&rig.fleet).save_teaching(teaching(&rig, Some(&session), TeachingReach::Repository, None)).await.expect("saved");

    eventually(|| async { rules(&rig, None).await == ["https://tracker.example.org/issues/{match}"] }).await;
    let file = words_file(&rig);
    assert!(file.contains("found = \"pull_request #41\""), "{file}");
}

#[tokio::test]
async fn where_nothing_is_found_the_person_is_asked_once_and_the_answer_is_the_address() {
    let rig = rig_fitted(|_| {});
    let session = rig.start().await;
    Arc::clone(&rig.fleet).save_teaching(teaching(&rig, Some(&session), TeachingReach::Repository, None)).await.expect("saved");

    eventually(|| async { calls(&rig, &session).await.len() == 1 }).await;
    let item = calls(&rig, &session).await.remove(0);
    let manifest = rig.fleet.manifest_id_at(&rig.root).expect("manifest");
    assert_eq!(item.id, format!("words:jira-key:{manifest}"));
    assert_eq!(item.text, format!("Where does Jira key open in {manifest}?"));
    assert_eq!(item.options.last().map(|one| one.label.as_str()), Some("Other"));
    assert_eq!(item.act.as_ref().map(|act| act.target.as_str()), Some(item.id.as_str()));

    // Asked again by every read, and still one question.
    for _ in 0..3 {
        rules(&rig, Some(&session)).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(calls(&rig, &session).await.len(), 1);

    let answer = |text: &str, choice: u32| AnswerWaiting {
        session_id: session.clone(),
        item_id: item.id.clone(),
        choice: Some(choice),
        text: Some(text.to_string()),
        mode: None,
    };
    let other = (item.options.len() - 1) as u32;
    let refused = Arc::clone(&rig.fleet).answer_waiting(answer("https://x.example/browse/", other)).await.expect_err("no {match}");
    assert_eq!(refused.status(), 422);
    assert_eq!(calls(&rig, &session).await.len(), 1, "a refused answer leaves the question standing");

    Arc::clone(&rig.fleet).answer_waiting(answer("https://x.example/browse/{match}", other)).await.expect("answered");
    assert!(calls(&rig, &session).await.is_empty());
    let file = words_file(&rig);
    assert!(file.contains("https://x.example/browse/{match}") && file.contains("found = \"person\""), "{file}");
    assert_eq!(rules(&rig, None).await, ["https://x.example/browse/{match}"]);
}

#[tokio::test]
async fn an_address_another_repository_has_is_offered_and_picking_it_writes_it() {
    let rig = rig_fitted(|_| {});
    let session = rig.start().await;
    Arc::clone(&rig.fleet).save_teaching(teaching(&rig, Some(&session), TeachingReach::Repository, None)).await.expect("saved");
    eventually(|| async { calls(&rig, &session).await.len() == 1 }).await;
    crate::teaching::address_given(&rig.fleet.mods_dir(), "jira-key", "elsewhere", "https://acme.example.net/browse/{match}", ipc::AddressFound::Person, true).expect("written");

    let item = calls(&rig, &session).await.remove(0);
    assert_eq!(item.options[0].label, "https://acme.example.net/browse/{match}");
    assert_eq!(item.options[0].description.as_deref(), Some("elsewhere"));
    Arc::clone(&rig.fleet)
        .answer_waiting(AnswerWaiting { session_id: session.clone(), item_id: item.id.clone(), choice: Some(0), text: None, mode: None })
        .await
        .expect("answered");
    assert_eq!(rules(&rig, None).await, ["https://acme.example.net/browse/{match}"]);
    assert!(calls(&rig, &session).await.is_empty());
}

#[tokio::test]
async fn a_question_put_aside_is_not_put_again_and_a_session_with_no_one_to_ask_asks_nobody() {
    let rig = rig_fitted(|_| {});
    let session = rig.start().await;
    let other = rig.start().await;
    Arc::clone(&rig.fleet).save_teaching(teaching(&rig, Some(&session), TeachingReach::Repository, None)).await.expect("saved");
    eventually(|| async { calls(&rig, &session).await.len() == 1 }).await;
    let item = calls(&rig, &session).await.remove(0);

    Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: session.clone(), item_id: item.id, reason: None, misread: false })
        .await
        .expect("dismissed");
    rules(&rig, Some(&session)).await;
    rules(&rig, Some(&other)).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(calls(&rig, &session).await.is_empty());
    assert!(calls(&rig, &other).await.is_empty(), "another Session is not asked what one put aside");
}
