//! Whether an agent's waiting item is clear to the person: a verb aimed at him and the thing it names,
//! no pointer elsewhere, not a status report. Every item here is written for the test, in the shapes
//! agents send; `docs/spikes/item-clarity.md` has the measurement over real ones.

use crate::session_host::clarity::{unclear, Unclear};

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
