//! A teaching read with one model call, nothing kept: the answer becomes a proposal the person
//! checks, and a link that could not work in Bridge is refused before they see it.

use std::sync::Arc;

use api::Refusal;
use ipc::{NoteKind, NoteSection, ReadTeaching, TeachingProposal, TeachingReach};
use testkit::FakeJudge;

use super::session_host::{rig_fitted, Rig};

const KEY: &str = r#"{"link":{"pattern":"[A-Z][A-Z0-9]+-\\d+","address":"https://myproject.jira.com/{match}"}}"#;

fn rig_saying(answer: &str) -> (Rig, Arc<FakeJudge>) {
    let judge = Arc::new(FakeJudge::saying(answer));
    let rig = rig_fitted(|fittings| fittings.judge = Arc::clone(&judge) as _);
    (rig, judge)
}

fn teaching(rig: &Rig, words: &str, said: &str) -> ReadTeaching {
    ReadTeaching {
        words: words.to_string(),
        message: format!("The ticket is {words}, see it."),
        said: said.to_string(),
        repository: Some(rig.root.clone()),
        reach: TeachingReach::Repository,
    }
}

async fn read(rig: &Rig, teach: ReadTeaching) -> Result<TeachingProposal, (u16, String)> {
    api::Teaching::read_teaching(&*rig.fleet, teach).await.map_err(|refusal: Refusal| (refusal.status(), refusal.error().code.clone()))
}

fn link_answer(pattern: &str, address: &str) -> String {
    format!(r#"{{"link":{{"pattern":{pattern:?},"address":{address:?}}}}}"#)
}

#[tokio::test]
async fn a_jira_key_is_proposed_as_a_link_with_the_selected_words_as_its_example() {
    let (rig, judge) = rig_saying(KEY);
    let said = "A Jira key. Make it a link: https://myproject.jira.com/STORE-1234";
    let proposal = read(&rig, teaching(&rig, "STORE-1234", said)).await.expect("read");

    let link = proposal.link.expect("a link");
    assert_eq!(link.pattern, r"[A-Z][A-Z0-9]+-\d+");
    assert_eq!(link.address, "https://myproject.jira.com/{match}");
    assert_eq!(link.example, "STORE-1234");
    assert!(proposal.note.is_none());
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one call");
    assert!(asked[0].contains("STORE-1234") && asked[0].contains(said), "the words and the person's words are put");
}

#[tokio::test]
async fn a_habit_is_proposed_as_a_note_for_every_repository() {
    let answer = r#"{"note":{"section":"habits","kind":"habit","text":"Propose an idea with a sketch and a short markdown list."}}"#;
    let (rig, _) = rig_saying(answer);
    let mut teach = teaching(&rig, "a plan", "Whenever you propose an idea like this, use a sketch and a short markdown list");
    teach.reach = TeachingReach::Everywhere;
    teach.repository = None;
    let note = read(&rig, teach).await.expect("read").note.expect("a note");
    assert_eq!((note.section, note.kind), (NoteSection::Habits, NoteKind::Habit));
    assert_eq!(note.text, "Propose an idea with a sketch and a short markdown list.");
}

#[tokio::test]
async fn an_answer_may_hold_both_a_link_and_a_note() {
    let answer = r#"{"link":{"pattern":"STORE-\\d+","address":"https://x.example/{match}"},"note":{"section":"repo","kind":"convention","text":"STORE keys are Jira tickets."}}"#;
    let (rig, _) = rig_saying(answer);
    let proposal = read(&rig, teaching(&rig, "STORE-1234", "A Jira key, and say so")).await.expect("read");
    assert!(proposal.link.is_some() && proposal.note.is_some());
}

#[tokio::test]
async fn a_link_that_cannot_work_in_bridge_is_refused_whole() {
    let https = "https://myproject.jira.com/{match}";
    let long = format!("STORE-1234{}", "x?".repeat(100));
    let refused = [
        ("a pattern missing the selected words", r"BLOG-\d+", https),
        ("a pattern matching only part of them", "[A-Z]+", https),
        ("a pattern that does not compile", "[A-Z", https),
        ("a lookahead", r"STORE(?=-1234)", https),
        ("a lookbehind", r"(?<=ST)ORE-1234", https),
        ("a backreference", r"(S)TORE-1234\1", https),
        ("a pattern that also matches nothing", r"(STORE-1234)?", https),
        ("an inline flag", r"(?i)store-1234", https),
        ("a repeated group holding a repeat", r"([A-Z]+)+-\d+", https),
        ("a repeated group holding a repeat of nothing or more", r"([A-Z]*)*-\d+", https),
        ("a repeated group holding a choice", r"(ST|S)+ORE-1234", https),
        ("a repeated group holding a repeat in a group", r"(([A-Z])+-)+\d+", https),
        ("a pattern over 200 characters", &long, https),
        ("an address that is not a web address", r"STORE-\d+", "ftp://myproject.example/{match}"),
        ("an address that runs a script", r"STORE-\d+", "javascript:alert({match})"),
        ("an address whose host is the match", r"STORE-\d+", "https://{match}.example/"),
        ("an address with no host", r"STORE-\d+", "https:///{match}"),
    ];
    for (why, pattern, address) in refused {
        let (rig, _) = rig_saying(&link_answer(pattern, address));
        let said = read(&rig, teaching(&rig, "STORE-1234", "A Jira key")).await;
        assert_eq!(said, Err((422, "fleet.teaching_link_refused".to_string())), "{why}");
    }
}

#[tokio::test]
async fn a_pattern_that_matches_the_selected_words_whole_is_not_refused_for_matching_others() {
    // UTF-8 would match too; only the selected words are held to it.
    let (rig, _) = rig_saying(KEY);
    assert!(read(&rig, teaching(&rig, "STORE-1234", "A Jira key")).await.is_ok());
    assert!(read(&rig, teaching(&rig, "UTF-8", "Not a ticket, but link it")).await.is_ok());
}

#[tokio::test]
async fn a_group_that_is_not_repeated_may_hold_repeats() {
    for pattern in [r"([A-Z][A-Z0-9]+)-(\d+)", r"(?:[A-Z]+)-\d+", r"[A-Z]+(-\d+)?"] {
        let (rig, _) = rig_saying(&link_answer(pattern, "https://myproject.jira.com/{match}"));
        assert!(read(&rig, teaching(&rig, "STORE-1234", "A Jira key")).await.is_ok(), "{pattern}");
    }
}

#[tokio::test]
async fn a_note_in_a_section_its_kind_or_its_reach_does_not_fit_is_refused() {
    let cases = [
        (r#"{"note":{"section":"habits","kind":"gotcha","text":"Text."}}"#, TeachingReach::Everywhere),
        (r#"{"note":{"section":"habits","kind":"habit","text":"Text."}}"#, TeachingReach::Repository),
        (r#"{"note":{"section":"repo","kind":"convention","text":"Text."}}"#, TeachingReach::Everywhere),
        (r#"{"note":{"section":"repo","kind":"convention","text":""}}"#, TeachingReach::Repository),
    ];
    for (answer, reach) in cases {
        let (rig, _) = rig_saying(answer);
        let mut teach = teaching(&rig, "a plan", "Remember this");
        teach.reach = reach;
        let said = read(&rig, teach).await;
        assert_eq!(said, Err((422, "fleet.teaching_note_refused".to_string())), "{answer}");
    }
}

#[tokio::test]
async fn an_answer_with_neither_is_refused_and_one_that_is_not_json_is_a_failed_call() {
    let (rig, _) = rig_saying(r#"{"link":null,"note":null}"#);
    assert_eq!(read(&rig, teaching(&rig, "x", "y")).await, Err((422, "fleet.teaching_nothing_read".to_string())));
    let (rig, _) = rig_saying("I could not tell what you meant.");
    assert_eq!(read(&rig, teaching(&rig, "x", "y")).await, Err((500, "fleet.teaching_read_failed".to_string())));
}

#[tokio::test]
async fn a_teaching_that_cannot_be_put_is_refused_before_the_model_is_asked() {
    let (rig, judge) = rig_saying(KEY);
    let mut empty = teaching(&rig, "STORE-1234", "   ");
    assert_eq!(read(&rig, empty.clone()).await, Err((422, "fleet.teaching_input_refused".to_string())));
    empty.said = "A Jira key".to_string();
    empty.words = " ".to_string();
    assert_eq!(read(&rig, empty).await, Err((422, "fleet.teaching_input_refused".to_string())));
    let mut unplaced = teaching(&rig, "STORE-1234", "A Jira key");
    unplaced.repository = None;
    assert_eq!(read(&rig, unplaced).await, Err((422, "fleet.teaching_input_refused".to_string())));
    let mut elsewhere = teaching(&rig, "STORE-1234", "A Jira key");
    elsewhere.repository = Some("/nowhere".to_string());
    assert_eq!(read(&rig, elsewhere).await, Err((404, "fleet.run_memory_repository_not_served".to_string())));
    assert!(judge.asked().is_empty(), "nothing was asked");
}
