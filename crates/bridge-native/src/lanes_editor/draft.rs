//! What an edit does to the text, where a draft applies, and the Changes list read off Fleet's drafts. The draft
//! itself is Fleet's (`WorkflowDraft`, `PieceDraft`): an edit here only works out the new whole text from the current
//! one, and the list is what the draft's changes say once the ones undone are left out.

use ipc::DraftChange;

use crate::settings::definition::{overlap, touched, with_delta, with_label, with_step_added, with_words};

/// What an edit does to the definition's text. Later changes (a rule, a step taken out) are more variants here and a
/// line in `apply`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// An agent's change, drafted from `before` as `after`. It carries only what differs between the two, so what
    /// the draft holds since stays.
    Definition { before: String, after: String },
    /// One step's words.
    StepWords { step: String, words: String },
    /// One step's name.
    StepLabel { step: String, label: String },
    /// A step added just after `after`, with empty words.
    AddStep { after: String, id: String, label: String },
    /// One state's name, in the lifecycle that holds it. A lifecycle is a piece of its own, not the workflow's
    /// text: the rename is kept as that lifecycle's draft.
    StateLabel { lifecycle: String, state: String, label: String },
}

impl Change {
    /// The text after the change, or none where it does not fit the text (a step it names is not there).
    pub fn apply(&self, text: &str) -> Option<String> {
        match self {
            Change::Definition { before, after } => with_delta(text, before, after),
            Change::StepWords { step, words } => with_words(text, step, words),
            Change::StepLabel { step, label } => with_label(text, step, label),
            Change::AddStep { after, id, label } => with_step_added(text, after, id, label),
            Change::StateLabel { .. } => Some(text.to_string()),
        }
    }
}

/// Where the draft is published: one repository's own copy, one project's, or Kit's for every project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A repository, by its Manifest id.
    Repository(String),
    /// A project, by its id: `WorkflowScope::Project`, for each repository it holds.
    Project(String),
    EveryProject,
}

/// A rename of a state that waits for where it applies: Armada's own workflow has no scope until it is answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waiting {
    pub said: String,
    pub lifecycle: String,
    pub state: String,
    pub label: String,
}

/// The summary a change carries that undoes another one in the middle of the list: `Undid #<id of the one undone>`.
/// Nothing displays it; the list reads it to leave both out.
pub fn undid_summary(id: i64) -> String {
    format!("Undid #{id}")
}

fn undid(change: &DraftChange) -> Option<i64> {
    change.summary.as_deref()?.strip_prefix("Undid #")?.parse().ok()
}

/// The text a change at `at` was made on: the change before it, or `origin` (what the draft began from).
pub fn before<'a>(changes: &'a [DraftChange], at: usize, origin: &'a str) -> &'a str {
    at.checked_sub(1).map_or(origin, |prev| changes[prev].text.as_str())
}

/// The changes the list shows, by their place in `changes`. A change whose text is one an earlier change left, or is
/// `origin`, takes back everything after that point and itself, so putting the previous text again shrinks the list.
/// A change that undoes one in the middle, and the one it names, are left out.
pub fn shown(changes: &[DraftChange], origin: &str) -> Vec<usize> {
    let mut rows: Vec<usize> = Vec::new();
    for (at, change) in changes.iter().enumerate() {
        if change.text == origin {
            rows.clear();
        } else if let Some(named) = undid(change) {
            rows.retain(|row| changes[*row].id != named);
        } else {
            match rows.iter().position(|row| changes[*row].text == change.text) {
                Some(was) => rows.truncate(was + 1),
                None => rows.push(at),
            }
        }
    }
    rows
}

/// Whether a change shown after `rows[k]` edits the same words: the fields each touched between the text before it and
/// its own overlap.
pub fn edited_later(changes: &[DraftChange], rows: &[usize], k: usize, origin: &str) -> bool {
    let touches = |at: usize| touched(before(changes, at, origin), &changes[at].text);
    let mine = touches(rows[k]);
    rows[k + 1..].iter().any(|later| touches(*later).iter().any(|theirs| mine.iter().any(|one| overlap(one, theirs))))
}

#[cfg(test)]
mod tests {
    use ipc::{DraftBy, Instant};

    use super::*;

    const BASE: &str = r#"{"version":1,"workflow_id":"bug","name":"bug","steps":[{"id":"fix","label":"Fix","instructions":"Fix it."},{"id":"tests","label":"Tests"}]}"#;

    fn steps(text: &str) -> Vec<(String, String)> {
        crate::settings::definition::steps_of(text)
    }

    fn step(id: &str, label: &str) -> (String, String) {
        (id.to_string(), label.to_string())
    }

    #[test]
    fn a_change_works_out_the_new_text_from_the_current_one() {
        let renamed = Change::StepLabel { step: "fix".into(), label: "Repair".into() }.apply(BASE).expect("fix is a step");
        let worded = Change::StepWords { step: "fix".into(), words: "Say why.".into() }.apply(&renamed).expect("fix is a step");
        assert_eq!(steps(&worded), [step("fix", "Repair"), step("tests", "Tests")]);
        assert_eq!(crate::settings::definition::words_of(&worded, "fix").as_deref(), Some("Say why."));
        assert_eq!(Change::StepWords { step: "gone".into(), words: "x".into() }.apply(&worded), None);
        let added = Change::AddStep { after: "fix".into(), id: "new_step".into(), label: "New step".into() }.apply(&worded).expect("added");
        assert_eq!(steps(&added), [step("fix", "Repair"), step("new_step", "New step"), step("tests", "Tests")]);
        assert_eq!(Change::StateLabel { lifecycle: "ticket".into(), state: "todo".into(), label: "Later".into() }.apply(&added).as_deref(), Some(added.as_str()), "a state is not the workflow's text");
    }

    #[test]
    fn an_agents_definition_carries_what_it_changed_on_top_of_what_the_draft_holds() {
        let after = BASE.replace("\"label\":\"Tests\"", "\"label\":\"Checks\"");
        let held = Change::StepLabel { step: "fix".into(), label: "Repair".into() }.apply(BASE).expect("fix");
        let landed = Change::Definition { before: BASE.into(), after }.apply(&held).expect("a delta");
        assert_eq!(steps(&landed), [step("fix", "Repair"), step("tests", "Checks")]);
    }

    fn change(id: i64, text: &str, summary: Option<&str>) -> DraftChange {
        DraftChange { id, by: DraftBy::You, at: Instant::carried("2026-11-13T09:00:00Z"), text: text.into(), summary: summary.map(String::from), change: None }
    }

    fn label(text: &str, step: &str, label: &str) -> String {
        with_label(text, step, label).expect("a step")
    }

    #[test]
    fn a_text_put_again_takes_back_the_rows_after_it_and_itself() {
        let (x, y) = (label(BASE, "fix", "Repair"), label(BASE, "tests", "Checks"));
        let both = label(&x, "tests", "Checks");
        let changes = [change(1, &x, None), change(2, &both, None)];
        assert_eq!(shown(&changes, BASE), [0, 1]);
        // Putting the first's text again drops the second and the put; putting the base drops everything.
        let back = [change(1, &x, None), change(2, &both, None), change(3, &x, None)];
        assert_eq!(shown(&back, BASE), [0]);
        let all = [change(1, &x, None), change(2, &both, None), change(3, &x, None), change(4, BASE, None)];
        assert!(shown(&all, BASE).is_empty());
        // The same text again, after a row that was taken back, is a row of its own.
        let again = [change(1, &x, None), change(2, &y, None), change(3, &x, None), change(4, &y, None)];
        assert_eq!(shown(&again, BASE), [0, 3]);
    }

    #[test]
    fn an_undo_in_the_middle_and_the_row_it_names_are_both_left_out() {
        let (x, both) = (label(BASE, "fix", "Repair"), label(&label(BASE, "fix", "Repair"), "tests", "Checks"));
        // Row 1 is undone from the middle: its reverse is put on top of row 2's text.
        let only_checks = label(BASE, "tests", "Checks");
        let changes = [change(1, &x, Some("Renamed Fix to Repair")), change(2, &both, Some("Renamed Tests to Checks")), change(3, &only_checks, Some("Undid #1"))];
        assert_eq!(shown(&changes, BASE), [1]);
        assert_eq!(shown(&changes[..2], BASE), [0, 1], "before the undo both are rows");
    }

    #[test]
    fn a_later_change_to_the_same_words_blocks_the_undo_of_an_earlier_one() {
        let first = crate::settings::definition::with_words(BASE, "fix", "Say why.").expect("fix");
        let second = crate::settings::definition::with_words(&first, "fix", "Say why, small.").expect("fix");
        let renamed = label(&second, "tests", "Checks");
        let changes = [change(1, &first, None), change(2, &second, None), change(3, &renamed, None)];
        let rows = shown(&changes, BASE);
        assert_eq!(rows, [0, 1, 2]);
        assert!(edited_later(&changes, &rows, 0, BASE), "the second edits the same step's words");
        assert!(!edited_later(&changes, &rows, 1, BASE), "the third is another step's name");
        assert!(!edited_later(&changes, &rows, 2, BASE));
    }
}
