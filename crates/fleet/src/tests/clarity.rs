//! Whether an agent's waiting item is clear to the person: a verb aimed at him and the thing it names,
//! no pointer elsewhere, not a status report. Every item here is written for the test, in the shapes
//! agents send; `docs/spikes/item-clarity.md` has the measurement over real ones.

use crate::session_host::clarity::{first_unclear, unclear, unclear_item_refusal, Unclear, IN_FORCE};

/// Items a person can act on as they stand, with whether a command goes with them.
const CLEAR: &[(&str, bool)] = &[
    ("Restart onto the preview so the walk window loads the strip fix", true),
    ("Merge #2126 once ci is green", false),
    ("Try the filter chip in the preview, then approve #2558", false),
    ("Should the page be read once at start, or on every open?", false),
    ("Which direction do I mock for the Cockpit?", false),
    ("Autopilot retries a failing call every few seconds: fix it here?", false),
    ("#2234 breaks two walks: who fixes them?", false),
    ("Mod 0.3.18 is on main: update both machines, then reload plugins", false),
    ("Free a worktree slot so I can fix the stuck subagents", false),
    ("Keep either of the two checks Fleet still posts on pull requests?", false),
    ("Stop the mock server left on port 51999", false),
    ("Update the armada mod to 0.3.22", true),
    ("Which way for session awareness across machines", false),
    ("Walk: the Ended list draws 50 rows at a time", false),
    ("Register the OAuth app for sign-in", false),
    ("Dispatcher picks: model plus effort, or a real second harness?", false),
    ("Five gaps the agents found: file them?", false),
    ("Optional: git worktree prune", false),
    ("Description card: pick A (plate) or B (prose) in the window", false),
];

/// Items he cannot act on: a heading read as a question, a status, a pointer, a bare noun.
const UNCLEAR: &[(&str, bool, Unclear)] = &[
    ("Bigger than it looked?", false, Unclear::NoAsk),
    ("Glyph for a file row", false, Unclear::NoAsk),
    ("probe", false, Unclear::NoAsk),
    ("Done", false, Unclear::Status),
    ("Tests pass", false, Unclear::Status),
    ("Tests pass and the branch is pushed", false, Unclear::Status),
    ("#2645 still in the merge queue, not yet in main", false, Unclear::Status),
    ("Waiting on #2650 to merge on green ci", false, Unclear::Status),
    ("PR #2642 merging on green ci", false, Unclear::Status),
    ("#2186 lands on green ci once you've seen the preview", false, Unclear::Status),
    ("Which of the options above do you want?", false, Unclear::Pointer),
    ("See above for the choices", false, Unclear::Pointer),
    ("Decide between the three options", false, Unclear::Pointer),
    ("Route a thought: 3 design choices before the mock", false, Unclear::Pointer),
    ("Approve it", false, Unclear::NoAsk),
    ("Merge this now", false, Unclear::NoAsk),
    ("Conflict detection: what happens on a real clash", false, Unclear::NoAsk),
    ("Merged and ready", false, Unclear::Status),
    ("Panel header", false, Unclear::NoAsk),
];

#[test]
fn an_item_with_a_verb_for_him_and_the_thing_it_names_is_clear() {
    for (text, runs) in CLEAR {
        assert_eq!(unclear(text, *runs), None, "{text}");
    }
}

#[test]
fn a_heading_a_status_a_pointer_or_a_bare_noun_is_not() {
    for (text, runs, why) in UNCLEAR {
        assert_eq!(unclear(text, *runs), Some(*why), "{text}");
    }
}

/// "Bigger than it looked?" was shown to the owner as his question: a `?` alone is not an ask.
#[test]
fn a_question_mark_alone_is_not_an_ask() {
    assert_eq!(unclear("Bigger than it looked?", false), Some(Unclear::NoAsk));
    assert_eq!(unclear("Is it bigger than it looked?", false), None);
}

/// Questions an agent asks every day open with a verb the list never held, or offer a choice: the
/// structure of the line names what he is to decide, not the word it opens with. These were refused
/// in the merge queue ("Sort by name or by version?") before the rule read structure.
#[test]
fn a_question_that_opens_with_a_verb_or_offers_a_choice_is_an_ask() {
    for text in [
        "Sort by name or by version?",
        "Go ahead with the plan?",
        "Squash the commits, or keep them?",
        "Roll the cache back to the old key?",
        "Ship it as one pull request or two?",
    ] {
        assert_eq!(unclear(text, false), None, "{text}");
    }
}

/// The same structure does not clear a question that opens with a subject, a comparison or a status,
/// nor a pointer.
#[test]
fn a_question_that_names_nothing_to_decide_stays_unclear() {
    assert_eq!(unclear("Bigger than it looked?", false), Some(Unclear::NoAsk));
    assert_eq!(unclear("Bigger than it looked on the second try?", false), Some(Unclear::NoAsk));
    assert_eq!(unclear("The tests pass on main?", false), Some(Unclear::Status));
    assert_eq!(unclear("See above.", false), Some(Unclear::Pointer));
    assert_eq!(unclear("Tests pass.", false), Some(Unclear::Status));
}

/// A command handed to him is the verb and the thing, so a short line with one is clear; a pointer is not.
#[test]
fn a_command_handed_over_makes_a_short_line_clear_but_not_a_pointer() {
    assert_eq!(unclear("Run this", true), None);
    assert_eq!(unclear("Run the command above", true), Some(Unclear::Pointer));
}

/// A status phrase beside a verb for him is still an ask: "Merge #12 once ci is green".
#[test]
fn a_status_phrase_beside_a_verb_for_him_is_an_ask() {
    assert_eq!(unclear("#12 is in the merge queue: approve the walk before it lands", false), None);
}

/// The list is read in order and the first unclear item is named, with the reason the rules gave.
#[test]
fn the_first_unclear_item_is_named_with_why_when_the_reader_is_in_force() {
    let list = [("tests", "Run the tests and tell me what fails", false), ("size", "Bigger than it looked?", false), ("done", "Tests pass", false)];
    assert_eq!(first_unclear(list, true), Some(("size", Unclear::NoAsk)));
    let clear = [("tests", "Run the tests and tell me what fails", false)];
    assert_eq!(first_unclear(clear, true), None);
}

#[test]
fn nothing_is_refused_where_the_reader_is_told_it_is_off() {
    assert_eq!(first_unclear([("size", "Bigger than it looked?", false)], false), None);
}

#[test]
fn the_reader_is_in_force() {
    assert!(IN_FORCE, "the owner picked it on 22 Oct 2026");
}

/// In the voice of the walk refusal: what was refused, why, what to do, and to call again.
#[test]
fn the_refusal_names_the_item_and_why_and_says_to_call_again() {
    let said = unclear_item_refusal("size", Unclear::NoAsk);
    assert!(said.starts_with("Refused: item \"size\""), "{said}");
    assert!(said.contains(Unclear::NoAsk.why()), "{said}");
    assert!(said.contains("call waiting_for again"), "{said}");
    for why in [Unclear::Pointer, Unclear::Status, Unclear::NoAsk] {
        assert!(unclear_item_refusal("x", why).contains(why.why()), "{why:?}");
    }
}

/// Through the list the agent sends: an item like "Bigger than it looked?" is refused, naming the item.
#[tokio::test]
async fn waiting_for_refuses_an_item_he_could_not_act_on() {
    use api::{Refusal, Sessions};
    use ipc::{SetWaitingFor, WaitingInput};

    use super::session_host::rig_placing;

    let rig = rig_placing(None);
    let id = rig.start().await;
    let set = SetWaitingFor {
        session_id: Some(id),
        items: vec![WaitingInput { id: "size".into(), text: "Bigger than it looked?".into(), act: None, options: Vec::new(), steps: Vec::new() }],
    };
    let refused = rig.fleet.waiting_for(None, set).await.expect_err("refused while in force");
    assert!(matches!(refused, Refusal::Unacceptable(_)), "{refused:?}");
}
