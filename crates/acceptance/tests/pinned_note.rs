//! A pinned note's claim: **a note the person pins on a page reaches the Session with the line of
//! code the spot was drawn from ranked first, found by the words on screen, and a stamp on the page
//! is believed only where the text bears it out.**
//!
//! The case is the owner's own note: the row "Other commands, 16.1% of the burn" on the Usage mock,
//! drawn by `ShareRows` at line 171 of `UsagePage.tsx`. Line 163 opens `ShareRows` and holds "of the
//! burn" too, and lines 145, 151, 341 and 534 hold it in a tooltip, so the words alone are not
//! enough: the row's own line has to beat them.
//!
//! | Proved | Not proved |
//! |---|---|
//! | With no stamp, line 171 ranks first, with why it matched | That the file is found in the checkout that served the page by `git grep`. `fleet::tests::session_notes` drives `send_session_note` over a real repository holding this file |
//! | A stamp at 171 ranks first, and says it was checked | That the Session's turn carries the candidates, the state and the crop while the thread keeps only the words. `fleet::tests::session_notes` |
//! | A stamp at a line holding none of the words is dropped, and 171 still ranks first | That Bridge sends a note this way. The TypeScript half is not built |

use fleet::note_locating::{located, StampRead};
use ipc::{NoteElement, NotePlace};

const PATH: &str = "packages/components/src/compositions/UsagePage/UsagePage.tsx";

/// A copy of the file as it stood on `usage/usage-page-mock` when the owner pinned the note.
const USAGE_PAGE: &str = include_str!("bench/pinned_note/UsagePage.tsx.txt");

fn files() -> Vec<(String, String)> {
    vec![(PATH.to_string(), USAGE_PAGE.to_string())]
}

fn place(stamp: Option<&str>) -> NotePlace {
    NotePlace {
        component: Some("ShareRows".to_string()),
        owners: vec!["UsagePage".to_string(), "Boundary".to_string()],
        selector: r#"ul[aria-label="Burn by Bash command"] > li[aria-label="Other commands, 16.1% of the burn"]"#.to_string(),
        element: NoteElement {
            tag: "li".to_string(),
            text: "Other commands 16.1%".to_string(),
            label: Some("Other commands, 16.1% of the burn".to_string()),
        },
        nearby: vec!["Burn by Bash command".to_string(), "Usage".to_string()],
        stamp: stamp.map(str::to_string),
        location: "/?walk=burnBySessionModelTool".to_string(),
        screen: None,
        layer: None,
    }
}

#[test]
fn the_rows_own_line_ranks_first_on_the_screens_words_alone() {
    let found = located(&place(None), &files());
    let first = found.candidates.first().expect("a line matched");
    assert_eq!((first.file.as_str(), first.line), (PATH, 171), "{:#?}", found.candidates);
    assert!(first.why.contains("of the burn"), "why names the words: {}", first.why);
    assert!(first.why.contains("ShareRows"), "why names the component: {}", first.why);
    assert!(first.text.starts_with("<li key={one.name}"), "{}", first.text);
    assert!(found.candidates.len() <= 5);
    assert_eq!(found.stamp, StampRead::Absent);
}

#[test]
fn a_stamp_the_text_bears_out_ranks_first_and_says_it_was_checked() {
    let found = located(&place(Some(&format!("{PATH}:171"))), &files());
    let first = found.candidates.first().expect("a line matched");
    assert_eq!((first.file.as_str(), first.line), (PATH, 171), "{:#?}", found.candidates);
    assert!(first.why.contains("stamp"), "why says the stamp was checked: {}", first.why);
    assert_eq!(found.stamp, StampRead::Checked);
    let again = found.candidates.iter().filter(|one| one.file == PATH && one.line == 171).count();
    assert_eq!(again, 1, "the stamped line is one candidate, not two");
}

#[test]
fn a_stamp_at_a_line_holding_none_of_the_words_is_dropped() {
    // Line 205 is `const W = 520;`, and nothing in the fifteen after it says what the row says.
    let found = located(&place(Some(&format!("{PATH}:205"))), &files());
    assert!(matches!(found.stamp, StampRead::Dropped(_)), "{:?}", found.stamp);
    assert!(found.candidates.iter().all(|one| one.line != 205), "{:#?}", found.candidates);
    let first = found.candidates.first().expect("a line matched");
    assert_eq!((first.file.as_str(), first.line), (PATH, 171), "{:#?}", found.candidates);
}
