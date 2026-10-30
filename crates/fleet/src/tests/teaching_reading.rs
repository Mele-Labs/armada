//! A teaching read with one model call, nothing kept: the answer becomes a proposal the person
//! checks, and each part of it that could not work where it will run is refused before they see it.

use std::sync::Arc;

use api::Refusal;
use ipc::{
    Condition, Does, Instead, LogbookKind, LogbookSection, PieceScope, ReadTeaching, RuleAct, RuleAmong, RuleCondition, RuleScope,
    RuleWhen, TeachingProposal, TeachingReach, TeachingSource, TeachingSourceKind,
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
    assert_eq!(
        words.address.map(|one| (one.template, one.found)),
        Some(("https://myproject.jira.com/browse/{match}".to_string(), ipc::AddressFound::Person))
    );
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

    let rule = proposal.fleet.expect("a rule").rule;
    assert_eq!((rule.notice.as_str(), rule.does, rule.scope), ("behind_base", Does::Batch, RuleScope::Repo(manifest)));
    assert!(proposal.note.is_some(), "a notice may teach an agent too");
    assert!(judge.asked()[0].contains("behind_base"), "the model is told which notice");

    let (rig, _) = rig_saying(r#"{"fleet":{"does":"quiet","where":"job"}}"#);
    let job = read(&rig, pressed(&rig, TeachingSourceKind::Notice, "behind", &["behind_base"])).await.expect("read").fleet.expect("rule").rule;
    assert_eq!(job.scope, RuleScope::Job("s1".to_string()), "the Job is the Session it was taught in");
    let (rig, _) = rig_saying(r#"{"fleet":{"does":"quiet","where":"everywhere"}}"#);
    let all = read(&rig, pressed(&rig, TeachingSourceKind::Notice, "behind", &["behind_base"])).await.expect("read").fleet.expect("rule").rule;
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

#[tokio::test]
async fn batch_these_but_tell_me_when_it_blocks_a_merge_is_a_batch_rule_with_that_condition() {
    let answer = r#"{"fleet":{"does":"batch","where":"everywhere","when":[{"if":"blocks_merge","does":"tell"}]}}"#;
    let (rig, judge) = rig_saying(answer);
    let pressed = pressed(&rig, TeachingSourceKind::Notice, "#7 failed its checks.", &["pr_checks_failed"]);
    let proposal = read(&rig, pressed).await.expect("read");

    let fleet = proposal.fleet.expect("a rule");
    assert_eq!(fleet.rule.does, Does::Batch);
    assert_eq!(fleet.rule.when, [Instead { condition: Condition::BlocksMerge, does: Does::Tell }]);
    assert!(fleet.rule.matches.is_none() && fleet.untestable.is_empty());
    let asked = &judge.asked()[0];
    for condition in ["blocks_merge", "open_pull_request", "working", "owner_away"] {
        assert!(asked.contains(condition), "the model is given the closed list, missing {condition}");
    }
}

#[tokio::test]
async fn a_condition_outside_the_set_comes_back_untestable_and_no_condition_is_made_up() {
    let answer = r#"{"fleet":{"does":"batch","where":"everywhere","untestable":["when it is a Friday"],"when":[{"if":"working","does":"tell"},{"if":"the build is red","does":"quiet"}]}}"#;
    let (rig, _) = rig_saying(answer);
    let pressed = pressed(&rig, TeachingSourceKind::Notice, "#7 failed its checks.", &["pr_checks_failed"]);
    let fleet = read(&rig, pressed).await.expect("read").fleet.expect("a rule");

    assert_eq!(fleet.rule.when, [Instead { condition: Condition::Working, does: Does::Tell }], "only a condition Fleet can test");
    assert_eq!(fleet.untestable, ["when it is a Friday", "the build is red"]);
    let wire = ipc::encode(&fleet).expect("plain data");
    assert!(wire.contains(r#""untestable":["when it is a Friday","the build is red"]"#) && wire.contains(r#""where":"everywhere""#), "{wire}");
}

#[tokio::test]
async fn a_pattern_over_the_notice_is_kept_only_if_it_works_and_finds_words_in_that_notice() {
    let text = "#7 failed its checks: lint";
    let said = |matches: &str| format!(r#"{{"fleet":{{"does":"quiet","where":"everywhere","matches":{matches:?}}}}}"#);
    let (rig, _) = rig_saying(&said("lint"));
    let kept = read(&rig, pressed(&rig, TeachingSourceKind::Notice, text, &["pr_checks_failed"])).await.expect("read");
    assert_eq!(kept.fleet.expect("a rule").rule.matches.as_deref(), Some("lint"), "found anywhere in the text, not whole");

    for bad in ["unit tests", "(a+)+", "(?i)lint", "a*", "["] {
        let (rig, _) = rig_saying(&said(bad));
        let refused = read(&rig, pressed(&rig, TeachingSourceKind::Notice, text, &["pr_checks_failed"])).await;
        assert_eq!(refused, Err((422, "fleet.teaching_rule_refused".to_string())), "{bad}");
    }
}

/// "When this is approved, move its tickets to Done", as a model would answer it for the built-in
/// lifecycles: `Done` is the ticket's `closed`.
const APPROVED_CLOSES_TICKETS: &str = r#"{"rule":{"name":"Close the tickets when a pull request is approved","when":{"kind":"enters","lifecycle":"pr","state":"approved"},"if":[],"do":[{"act":"tracker_move","to":"closed","target":{"among":{"kind":"linked_from","lifecycle":"ticket"}}}],"autonomy":null,"on_failure":null}}"#;

fn rule_answer(rule: &str) -> String {
    format!(r#"{{"rule":{rule}}}"#)
}

async fn rule_refused(answer: &str, why: &str) {
    let (rig, _) = rig_saying(answer);
    let said = read(&rig, pressed(&rig, TeachingSourceKind::PullRequest, "#812 Add the store page", &["812"])).await;
    assert_eq!(said, Err((422, "fleet.teaching_rule_refused".to_string())), "{why}");
}

#[tokio::test]
async fn a_pull_request_teaching_proposes_a_rule_placed_by_the_reach_and_named_from_its_name() {
    let (rig, judge) = rig_saying(APPROVED_CLOSES_TICKETS);
    let manifest = rig.fleet.manifest_id_at(&rig.root).expect("the rig's repository has a manifest");
    let mut teach = pressed(&rig, TeachingSourceKind::PullRequest, "#812 Add the store page", &["812"]);
    teach.said = "When this is approved, move its tickets to Done".to_string();
    let proposal = read(&rig, teach).await.expect("read");

    let rule = proposal.rule.expect("a rule").rule;
    assert_eq!(rule.id, "close_the_tickets_when_a_pull_request_is_approved");
    assert_eq!(rule.scope, PieceScope::Within { id: manifest }, "the repository reach");
    assert_eq!(rule.when, RuleWhen::Enters { lifecycle: "pr".to_string(), state: "approved".to_string() });
    assert!(matches!(&rule.does[0].what, RuleAct::TrackerMove { to, target } if to == "closed" && target.among == RuleAmong::LinkedFrom { lifecycle: "ticket".to_string() }));
    assert_eq!((rule.does[0].kind.as_str(), rule.does[0].autonomy.as_str()), ("tracker_write", "asks"), "worked out as a read of the file works it out");
    assert_eq!((rule.autonomy, rule.on_failure.retries, rule.on_failure.then.as_str()), (None, 2, "ask_owner"), "the defaults a file would get");
    assert!(proposal.note.is_none() && proposal.words.is_none());

    let asked = &judge.asked()[0];
    assert!(asked.contains("ticket (Ticket): todo (To do), in_progress (In progress), in_review (In review), closed (Closed)"), "{asked}");
    assert!(asked.contains("pr (Pull request): draft (Draft)") && asked.contains("approved (Approved)"), "the states are named");
    assert!(asked.contains("Workflows, each with its steps"), "the workflows are listed");
    for act in ["tracker_move", "forge_merge", "start_watch", "run_drone", "wake"] {
        assert!(asked.contains(act), "the acts Fleet runs are named, missing {act}");
    }
}

#[tokio::test]
async fn the_steps_of_the_repositorys_workflows_are_given_to_the_model() {
    let (rig, judge) = rig_saying(APPROVED_CLOSES_TICKETS);
    let listed = api::Queries::list_workflows(&*rig.fleet).await.expect("listed");
    let workflow = listed.first().expect("the rig holds a workflow");
    let step = workflow.steps.first().expect("with a step");
    read(&rig, pressed(&rig, TeachingSourceKind::PullRequest, "#812", &["812"])).await.expect("read");
    let asked = &judge.asked()[0];
    assert!(asked.contains(&format!("- {} ({}): ", workflow.id.as_str(), workflow.name)), "{asked}");
    assert!(asked.contains(&format!("{} ({})", step.step_id.as_str(), step.label)), "{asked}");
}

#[tokio::test]
async fn a_rule_for_every_repository_is_at_the_default_level() {
    let (rig, _) = rig_saying(APPROVED_CLOSES_TICKETS);
    let mut teach = pressed(&rig, TeachingSourceKind::PullRequest, "#812", &["812"]);
    teach.reach = TeachingReach::Everywhere;
    teach.repository = None;
    let rule = read(&rig, teach).await.expect("read").rule.expect("a rule").rule;
    assert_eq!(rule.scope, PieceScope::Everywhere);
}

#[tokio::test]
async fn a_notice_and_a_ticket_may_make_a_rule_and_a_ticket_may_make_a_note_but_not_words() {
    let both = r#"{"rule":{"name":"Tell the agent","when":{"kind":"job","event":"pull_request_opened"},"do":[{"act":"tell","message":"Say when the checks pass."}]},"note":{"section":"repo","kind":"convention","text":"Tickets close when their pull request is approved."},"words":{"name":"Key","pattern":"STORE-\\d+","address":null}}"#;
    for (kind, text, ids) in [(TeachingSourceKind::Ticket, "STORE-12 Add the store page", &["STORE-12"][..]), (TeachingSourceKind::Notice, "#7 is approved", &["pr_checks_failed"][..])] {
        let (rig, _) = rig_saying(both);
        let proposal = read(&rig, pressed(&rig, kind, text, ids)).await.expect("read");
        assert!(proposal.rule.is_some() && proposal.note.is_some(), "{kind:?}");
        assert!(proposal.words.is_none(), "{kind:?} makes no words");
    }
    let (rig, judge) = rig_saying(both);
    read(&rig, pressed(&rig, TeachingSourceKind::Ticket, "STORE-12 Add the store page", &["STORE-12"])).await.expect("read");
    assert!(judge.asked()[0].contains("a ticket in a Job's lanes") && judge.asked()[0].contains("STORE-12 Add the store page"));
}

#[tokio::test]
async fn a_source_that_cannot_make_a_rule_drops_it_and_an_answer_of_only_a_rule_is_nothing() {
    for kind in [TeachingSourceKind::Words, TeachingSourceKind::Command, TeachingSourceKind::Task, TeachingSourceKind::Check, TeachingSourceKind::Row] {
        let (rig, _) = rig_saying(APPROVED_CLOSES_TICKETS);
        let said = read(&rig, pressed(&rig, kind, "something", &[])).await;
        assert_eq!(said, Err((422, "fleet.teaching_nothing_read".to_string())), "{kind:?}");
    }
    let (rig, judge) = rig_saying(APPROVED_CLOSES_TICKETS);
    read(&rig, pressed(&rig, TeachingSourceKind::Check, "lint", &[])).await.ok();
    assert!(!judge.asked()[0].contains("tracker_move"), "the model is not offered a rule it may not make");
}

#[tokio::test]
async fn a_condition_fleet_cannot_test_comes_back_untestable_and_none_is_made_up() {
    let answer = rule_answer(
        r#"{"name":"Merge when approved","when":{"kind":"enters","lifecycle":"pr","state":"approved"},"if":[{"kind":"pull_request_opened"},"when the build is green"],"do":[{"act":"forge_merge","target":{"among":{"kind":"entered"}}}],"untestable":["when it is not a Friday"]}"#,
    );
    let (rig, _) = rig_saying(&answer);
    let proposal = read(&rig, pressed(&rig, TeachingSourceKind::PullRequest, "#812", &["812"])).await.expect("read");
    let rule = proposal.rule.expect("a rule");
    assert_eq!(rule.rule.conditions, [RuleCondition::PullRequestOpened], "only a condition Fleet can test");
    assert_eq!(rule.untestable, ["when it is not a Friday", "when the build is green"]);
    let wire = ipc::encode(&rule).expect("plain data");
    assert!(wire.contains(r#""untestable":["when it is not a Friday","when the build is green"]"#) && wire.contains(r#""id":"merge_when_approved""#), "{wire}");
}

#[tokio::test]
async fn a_rule_a_rule_piece_would_not_load_is_refused() {
    let enters = r#""when":{"kind":"enters","lifecycle":"pr","state":"approved"}"#;
    let moves = r#"{"act":"tracker_move","to":"closed","target":{"among":{"kind":"linked_from","lifecycle":"ticket"}}}"#;
    let cases = [
        ("a state its lifecycle lacks", format!(r#"{{"name":"A","when":{{"kind":"enters","lifecycle":"pr","state":"aproved"}},"do":[{moves}]}}"#)),
        ("a lifecycle that does not apply", format!(r#"{{"name":"A","when":{{"kind":"enters","lifecycle":"release","state":"out"}},"do":[{moves}]}}"#)),
        ("a state of the ticket that is not there", format!(r#"{{"name":"A",{enters},"do":[{{"act":"tracker_move","to":"done","target":{{"among":{{"kind":"linked_from","lifecycle":"ticket"}}}}}}]}}"#)),
        ("nothing to do", format!(r#"{{"name":"A",{enters},"do":[]}}"#)),
        ("the entered instance of a job event", r#"{"name":"A","when":{"kind":"job","event":"landed"},"do":[{"act":"forge_merge","target":{"among":{"kind":"entered"}}}]}"#.to_string()),
        ("a quantity it has no word for", format!(r#"{{"name":"A",{enters},"if":[{{"kind":"instances","among":{{"kind":"held","lifecycle":"ticket"}},"quantity":"most","in_state":"closed"}}],"do":[{moves}]}}"#)),
        ("a word for autonomy it lacks", format!(r#"{{"name":"A",{enters},"do":[{moves}],"autonomy":"sometimes"}}"#)),
        ("more retries than the cap", format!(r#"{{"name":"A",{enters},"do":[{moves}],"on_failure":{{"retries":9,"then":"ask_owner"}}}}"#)),
        ("reviewers from nobody", format!(r#"{{"name":"A",{enters},"do":[{{"act":"forge_request_reviewers","from":[],"target":{{"among":{{"kind":"entered"}}}}}}]}}"#)),
        ("a blank name", format!(r#"{{"name":"  ",{enters},"do":[{moves}]}}"#)),
    ];
    for (why, rule) in cases {
        rule_refused(&rule_answer(&rule), why).await;
    }
}

#[tokio::test]
async fn the_id_is_made_from_the_name_and_a_rule_beside_it_is_not_replaced() {
    let rule = |name: &str| rule_answer(&format!(r#"{{"name":{name:?},"when":{{"kind":"enters","lifecycle":"pr","state":"approved"}},"do":[{{"act":"forge_merge","target":{{"among":{{"kind":"entered"}}}}}}]}}"#));
    let (rig, _) = rig_saying(&rule("2 tickets: Done!"));
    let first = read(&rig, pressed(&rig, TeachingSourceKind::PullRequest, "#812", &["812"])).await.expect("read").rule.expect("rule").rule;
    assert_eq!(first.id, "rule_2_tickets_done", "an id starts with a letter");

    let folder = std::path::Path::new(&rig.fleet.host().kit_home).join("rules");
    std::fs::create_dir_all(&folder).expect("made");
    std::fs::write(
        folder.join("mine.yml"),
        "version: 1\nrule_id: rule_2_tickets_done\nname: Mine\nscope: elsewhere\nwhen: { enters: { lifecycle: pr, state: merged } }\ndo:\n  - tell_agent: { wake: true }\n",
    )
    .expect("written");
    let second = read(&rig, pressed(&rig, TeachingSourceKind::PullRequest, "#812", &["812"])).await.expect("read").rule.expect("rule").rule;
    assert_eq!(second.id, "rule_2_tickets_done_2", "an id Kit already keeps, at any scope, is taken");
}
