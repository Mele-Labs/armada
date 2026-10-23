//! A teaching read with one model call, nothing kept: the answer becomes a proposal the person
//! checks, and each part of it that could not work where it will run is refused before they see it.

use std::sync::Arc;

use api::Refusal;
use ipc::{
    Does, LogbookKind, LogbookSection, ReadTeaching, RuleScope, TeachingProposal, TeachingReach, TeachingSource,
    TeachingSourceKind,
};
use testkit::FakeJudge;

use super::session_host::{rig_fitted, Rig};

const KEY: &str = r#"{"words":{"name":"Jira key","pattern":"[A-Z][A-Z0-9]+-\\d+","address":"https://myproject.jira.com/browse/{match}"}}"#;

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
        session_id: Some("s1".to_string()),
        source: None,
    }
}

fn pressed(rig: &Rig, kind: TeachingSourceKind, text: &str, ids: &[&str]) -> ReadTeaching {
    ReadTeaching {
        words: String::new(),
        message: String::new(),
        source: Some(TeachingSource { kind, text: text.to_string(), ids: ids.iter().map(|id| id.to_string()).collect() }),
        ..teaching(rig, "", "Make this different")
    }
}

async fn read(rig: &Rig, teach: ReadTeaching) -> Result<TeachingProposal, (u16, String)> {
    api::Teaching::read_teaching(&*rig.fleet, teach).await.map_err(|refusal: Refusal| (refusal.status(), refusal.error().code.clone()))
}

fn words_answer(pattern: &str, address: &str) -> String {
    format!(r#"{{"words":{{"name":"Key","pattern":{pattern:?},"address":{address:?}}}}}"#)
}

#[tokio::test]
async fn a_jira_key_is_proposed_as_words_with_the_selected_words_as_its_example() {
    let (rig, judge) = rig_saying(KEY);
    let said = "A Jira key. Make it a link: https://myproject.jira.com/browse/STORE-1234";
    let proposal = read(&rig, teaching(&rig, "STORE-1234", said)).await.expect("read");

    let words = proposal.words.expect("words");
    assert_eq!((words.name.as_str(), words.pattern.as_str()), ("Jira key", r"[A-Z][A-Z0-9]+-\d+"));
    assert_eq!(words.address.as_deref(), Some("https://myproject.jira.com/browse/{match}"));
    assert_eq!(words.example, "STORE-1234");
    assert!(proposal.note.is_none() && proposal.fleet.is_none() && proposal.tweak.is_empty() && proposal.layout.is_none());
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one call");
    assert!(asked[0].contains("STORE-1234") && asked[0].contains(said), "the words and the person's words are put");
    assert!(!asked[0].contains("--bg-raised"), "tokens are for a teaching on Bridge's interface only");
}

#[tokio::test]
async fn words_the_person_gave_no_address_for_carry_none() {
    let answer = r#"{"words":{"name":"Jira key","pattern":"[A-Z][A-Z0-9]+-\\d+","address":null}}"#;
    let (rig, _) = rig_saying(answer);
    let words = read(&rig, teaching(&rig, "STORE-1234", "A Jira key")).await.expect("read").words.expect("words");
    assert_eq!(words.address, None);
}

#[tokio::test]
async fn a_habit_is_proposed_as_a_note_for_every_repository() {
    let answer = r#"{"note":{"section":"habits","kind":"habit","text":"Propose an idea with a sketch and a short markdown list."}}"#;
    let (rig, _) = rig_saying(answer);
    let mut teach = teaching(&rig, "a plan", "Whenever you propose an idea like this, use a sketch and a short markdown list");
    teach.reach = TeachingReach::Everywhere;
    teach.repository = None;
    let note = read(&rig, teach).await.expect("read").note.expect("a note");
    assert_eq!((note.section, note.kind), (LogbookSection::Habits, LogbookKind::Habit));
    assert_eq!(note.text, "Propose an idea with a sketch and a short markdown list.");
}

#[tokio::test]
async fn an_answer_may_hold_both_words_and_a_note() {
    let answer = r#"{"words":{"name":"Store key","pattern":"STORE-\\d+","address":"https://x.example/{match}"},"note":{"section":"repo","kind":"convention","text":"STORE keys are Jira tickets."}}"#;
    let (rig, _) = rig_saying(answer);
    let proposal = read(&rig, teaching(&rig, "STORE-1234", "A Jira key, and say so")).await.expect("read");
    assert!(proposal.words.is_some() && proposal.note.is_some());
}

#[tokio::test]
async fn words_that_cannot_work_in_bridge_are_refused_whole() {
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
        ("an address with nowhere for the match", r"STORE-\d+", "https://myproject.jira.com/browse/"),
    ];
    for (why, pattern, address) in refused {
        let (rig, _) = rig_saying(&words_answer(pattern, address));
        let said = read(&rig, teaching(&rig, "STORE-1234", "A Jira key")).await;
        assert_eq!(said, Err((422, "fleet.teaching_words_refused".to_string())), "{why}");
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
        let (rig, _) = rig_saying(&words_answer(pattern, "https://myproject.jira.com/{match}"));
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
async fn an_answer_with_nothing_is_refused_and_one_that_is_not_json_is_a_failed_call() {
    let (rig, _) = rig_saying(r#"{"words":null,"note":null}"#);
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
    let blank_panel = pressed(&rig, TeachingSourceKind::Panel, "  ", &["merge-line"]);
    assert_eq!(read(&rig, blank_panel).await, Err((422, "fleet.teaching_input_refused".to_string())));
    assert!(judge.asked().is_empty(), "nothing was asked");
}

const WARMER: &str = r##"{"tweak":[{"token":"--bg-raised","now":"#1F1A15"}],"note":{"section":"habits","kind":"habit","text":"Keep panels warm."}}"##;

#[tokio::test]
async fn a_panel_teaching_proposes_a_tweak_with_the_shipped_value_and_what_moves_with_it() {
    let (rig, judge) = rig_saying(WARMER);
    let mut teach = pressed(&rig, TeachingSourceKind::Panel, "The Cockpit's merge line panel", &["merge-line"]);
    teach.reach = TeachingReach::Everywhere;
    let proposal = read(&rig, teach).await.expect("read");

    let change = &proposal.tweak[0];
    assert_eq!((change.token.as_str(), change.now.as_str()), ("--bg-raised", "#1F1A15"));
    assert_eq!(change.was, "#161C23", "the shipped value is Fleet's to say, not the model's");
    assert!(change.used_in.contains(&"--surface-card".to_string()), "the aliases move with it: {:?}", change.used_in);
    assert_eq!(change.note.as_deref(), Some("cards, table rows, panels"));
    assert!(proposal.note.is_none(), "Bridge's own interface has no agent note");
    let asked = judge.asked();
    assert!(asked[0].contains("--bg-raised #161C23 (cards, table rows, panels)"), "the model chooses from the tokens");
    assert!(asked[0].contains("merge-line") && asked[0].contains("hideable"), "and from the layout registry");
}

#[tokio::test]
async fn a_tweak_the_stylesheet_checks_refuse_is_refused() {
    let cases = [
        r#"{"tweak":[{"token":"--not-a-token","now":"red"}]}"#,
        r#"{"tweak":[{"token":"--accent","now":"url(https://x.example/a.png)"}]}"#,
        r#"{"tweak":[{"token":"--accent","now":"red; color: blue"}]}"#,
        r#"{"tweak":[{"token":"--accent","now":""}]}"#,
        r#"{"tweak":[]}"#,
    ];
    for answer in cases {
        let (rig, _) = rig_saying(answer);
        let said = read(&rig, pressed(&rig, TeachingSourceKind::Row, "A row", &[])).await;
        assert_eq!(said, Err((422, "fleet.teaching_tweak_refused".to_string())), "{answer}");
    }
}

#[tokio::test]
async fn a_layout_is_proposed_as_the_text_a_layout_mod_holds_and_one_the_registry_refuses_is_refused() {
    let tidy = r#"{"layout":{"name":"Tidy","regions":{"rail":{"hidden":["lessons"]},"dashboard.panels":{"hidden":["merge-line"]}}}}"#;
    let (rig, _) = rig_saying(tidy);
    let proposal = read(&rig, pressed(&rig, TeachingSourceKind::TitleBar, "The title bar", &[])).await.expect("read");
    let layout = proposal.layout.expect("a layout");
    assert_eq!(layout.name, "Tidy");
    assert_eq!(layout.layout, r#"{"version":1,"dashboard.panels":{"hidden":["merge-line"]},"rail":{"hidden":["lessons"]}}"#);
    assert!(ipc::layout::problems(&layout.layout).is_empty());

    for bad in [
        r#"{"layout":{"name":"Bad","regions":{"rail":{"first":"Not A Slug"}}}}"#,
        r#"{"layout":{"name":"Invented","regions":{"sidebar":{"hidden":["lessons"]}}}}"#,
        r#"{"layout":{"name":"Empty","regions":{}}}"#,
        r#"{"layout":{"name":"","regions":{"rail":{"hidden":["lessons"]}}}}"#,
    ] {
        let (rig, _) = rig_saying(bad);
        let said = read(&rig, pressed(&rig, TeachingSourceKind::Panel, "A panel", &["fleet"])).await;
        assert_eq!(said, Err((422, "fleet.teaching_layout_refused".to_string())), "{bad}");
    }
}

#[tokio::test]
async fn a_notice_teaching_proposes_a_rule_for_the_notice_it_was_pressed_on() {
    let answer = r#"{"fleet":{"does":"batch","where":"repo"},"note":{"section":"repo","kind":"convention","text":"Slots fall behind; do not rebase unasked."}}"#;
    let (rig, judge) = rig_saying(answer);
    let manifest = rig.fleet.manifest_id_at(&rig.root).expect("the rig's repository has a manifest");
    let proposal = read(&rig, pressed(&rig, TeachingSourceKind::Notice, "slot-3 is 12 commits behind", &["behind_base"])).await.expect("read");

    let rule = proposal.fleet.expect("a rule");
    assert_eq!((rule.notice.as_str(), rule.does, rule.scope), ("behind_base", Does::Batch, RuleScope::Repo(manifest)));
    assert!(proposal.note.is_some(), "a notice may teach an agent too");
    assert!(judge.asked()[0].contains("behind_base"), "the model is told which notice");

    let (rig, _) = rig_saying(r#"{"fleet":{"does":"quiet","where":"job"}}"#);
    let job = read(&rig, pressed(&rig, TeachingSourceKind::Notice, "behind", &["behind_base"])).await.expect("read").fleet.expect("rule");
    assert_eq!(job.scope, RuleScope::Job("s1".to_string()), "the Job is the Session it was taught in");
    let (rig, _) = rig_saying(r#"{"fleet":{"does":"quiet","where":"everywhere"}}"#);
    let all = read(&rig, pressed(&rig, TeachingSourceKind::Notice, "behind", &["behind_base"])).await.expect("read").fleet.expect("rule");
    assert_eq!(all.scope, RuleScope::Everywhere);
}

#[tokio::test]
async fn a_rule_for_a_notice_fleet_does_not_send_or_with_no_place_to_name_is_refused() {
    let (rig, _) = rig_saying(r#"{"fleet":{"does":"quiet","where":"everywhere"}}"#);
    let unknown = pressed(&rig, TeachingSourceKind::Notice, "a notice", &["made_up"]);
    assert_eq!(read(&rig, unknown).await, Err((422, "fleet.teaching_rule_refused".to_string())));
    let (rig, _) = rig_saying(r#"{"fleet":{"does":"quiet","where":"job"}}"#);
    let mut unplaced = pressed(&rig, TeachingSourceKind::Notice, "behind", &["behind_base"]);
    unplaced.session_id = None;
    assert_eq!(read(&rig, unplaced).await, Err((422, "fleet.teaching_rule_refused".to_string())));
}

#[tokio::test]
async fn a_part_the_source_cannot_make_is_dropped_and_an_answer_of_only_such_parts_is_nothing() {
    let (rig, _) = rig_saying(r#"{"fleet":{"does":"quiet","where":"everywhere"},"words":{"name":"Key","pattern":"STORE-\\d+","address":null}}"#);
    let proposal = read(&rig, teaching(&rig, "STORE-1234", "A key")).await.expect("read");
    assert!(proposal.fleet.is_none() && proposal.words.is_some(), "a rule needs a notice to be pressed on");
    let (rig, _) = rig_saying(r#"{"fleet":{"does":"quiet","where":"everywhere"}}"#);
    assert_eq!(read(&rig, teaching(&rig, "STORE-1234", "A key")).await, Err((422, "fleet.teaching_nothing_read".to_string())));
}
