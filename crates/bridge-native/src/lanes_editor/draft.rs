//! The draft: a workflow's definition as it was read (`base`) and every edit since, in order. What the canvas
//! shows and what Publish writes is the base with each edit replayed, never a copy that is edited in place, so
//! dropping any one edit is dropping it from the list and replaying the rest.

use crate::settings::definition::{with_delta, with_label, with_step_added, with_words};

/// Whose edit it is: the person's own, or an agent's that was asked for in the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Author {
    Person,
    Agent,
}

/// What an edit does to the definition's text. Later changes (a rule, a step taken out) are more variants here and a
/// line in `apply`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// An agent's change, drafted from `before` as `after`. It carries only what differs between the two, so the
    /// person's edits before it stay, and dropping one of his replays the agent's on top of what is left.
    Definition { before: String, after: String },
    /// One step's words.
    StepWords { step: String, words: String },
    /// One step's name.
    StepLabel { step: String, label: String },
    /// A step added just after `after`, with empty words.
    AddStep { after: String, id: String, label: String },
}

impl Change {
    /// The text after the change, or none where it does not fit the text (a step it names is not there).
    pub fn apply(&self, text: &str) -> Option<String> {
        match self {
            Change::Definition { before, after } => with_delta(text, before, after),
            Change::StepWords { step, words } => with_words(text, step, words),
            Change::StepLabel { step, label } => with_label(text, step, label),
            Change::AddStep { after, id, label } => with_step_added(text, after, id, label),
        }
    }
}

/// One change in the history: who made it, the line the history reads, and the change itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub author: Author,
    pub said: String,
    pub change: Change,
}

/// Where the draft is published: one repository's own copy, or Kit's for every project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A repository, by its Manifest id.
    Repository(String),
    EveryProject,
}

/// The base with each edit replayed in order. An edit that does not fit what the ones before it left is skipped.
pub fn replay(base: &str, edits: &[Edit]) -> String {
    edits.iter().fold(base.to_string(), |text, edit| edit.change.apply(&text).unwrap_or(text))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    base: String,
    target: Option<Target>,
    edits: Vec<Edit>,
}

impl Draft {
    pub fn new(base: impl Into<String>, target: Option<Target>) -> Draft {
        Draft { base: base.into(), target, edits: Vec::new() }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    pub fn set_target(&mut self, target: Option<Target>) {
        self.target = target;
    }

    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    pub fn push(&mut self, edit: Edit) {
        self.edits.push(edit);
    }

    /// The last edit dropped (⌘Z).
    pub fn undo_last(&mut self) -> Option<Edit> {
        self.edits.pop()
    }

    /// One edit dropped, wherever it stands; the rest replay without it.
    pub fn undo_at(&mut self, at: usize) -> Option<Edit> {
        (at < self.edits.len()).then(|| self.edits.remove(at))
    }

    /// Every edit dropped, and the base taken afresh.
    pub fn clear(&mut self, base: impl Into<String>) {
        self.edits.clear();
        self.base = base.into();
    }

    /// The definition as it reads with every edit in it.
    pub fn drafted(&self) -> String {
        replay(&self.base, &self.edits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"{"version":1,"workflow_id":"bug","name":"bug","steps":[{"id":"fix","label":"Fix","instructions":"Fix it."},{"id":"tests","label":"Tests"}]}"#;

    fn words(step: &str, words: &str) -> Edit {
        Edit { author: Author::Person, said: format!("{step} words"), change: Change::StepWords { step: step.into(), words: words.into() } }
    }

    fn label(step: &str, label: &str) -> Edit {
        Edit { author: Author::Person, said: format!("{step} name"), change: Change::StepLabel { step: step.into(), label: label.into() } }
    }

    fn drafted(draft: &Draft) -> (Vec<(String, String)>, Vec<Option<String>>) {
        let text = draft.drafted();
        let steps = crate::settings::definition::steps_of(&text);
        let words = steps.iter().map(|(id, _)| crate::settings::definition::tree_of(&text).and_then(|tree| match tree.get("steps") {
            Some(crate::settings::definition::Json::List(list)) => list.iter().find(|one| one.get("id").and_then(crate::settings::definition::Json::text) == Some(id.as_str())).and_then(|one| one.get("instructions").and_then(crate::settings::definition::Json::text).map(String::from)),
            _ => None,
        })).collect();
        (steps, words)
    }

    #[test]
    fn no_edits_is_the_base_and_every_edit_replays_in_order() {
        let mut draft = Draft::new(BASE, None);
        assert_eq!(draft.drafted(), BASE);
        draft.push(label("fix", "Repair"));
        draft.push(words("fix", "Make the smallest change."));
        draft.push(label("tests", "Checks"));
        let (steps, words) = drafted(&draft);
        assert_eq!(steps, [("fix".to_string(), "Repair".to_string()), ("tests".to_string(), "Checks".to_string())]);
        assert_eq!(words, [Some("Make the smallest change.".to_string()), None]);
    }

    #[test]
    fn the_last_edit_is_dropped_and_what_stood_before_it_stands() {
        let mut draft = Draft::new(BASE, None);
        draft.push(label("fix", "Repair"));
        draft.push(label("fix", "Mend"));
        assert_eq!(draft.undo_last().map(|edit| edit.said), Some("fix name".to_string()));
        assert_eq!(drafted(&draft).0[0].1, "Repair");
        draft.undo_last();
        assert_eq!(draft.drafted(), BASE);
        assert_eq!(draft.undo_last(), None);
    }

    #[test]
    fn a_middle_edit_is_dropped_and_the_rest_replay_without_it() {
        let mut draft = Draft::new(BASE, None);
        draft.push(label("fix", "Repair"));
        draft.push(words("fix", "Say why."));
        draft.push(label("tests", "Checks"));
        assert_eq!(draft.undo_at(1).map(|edit| edit.said), Some("fix words".to_string()));
        let (steps, words) = drafted(&draft);
        assert_eq!(steps, [("fix".to_string(), "Repair".to_string()), ("tests".to_string(), "Checks".to_string())]);
        assert_eq!(words, [Some("Fix it.".to_string()), None]);
        assert_eq!(draft.edits().len(), 2);
        assert_eq!(draft.undo_at(5), None);
    }

    #[test]
    fn an_agents_definition_carries_what_it_changed_from_what_it_was_drafted_from() {
        let after = BASE.replace("\"label\":\"Tests\"", "\"label\":\"Checks\"");
        let mut draft = Draft::new(BASE, None);
        draft.push(Edit { author: Author::Agent, said: "Rename Tests".into(), change: Change::Definition { before: BASE.into(), after: after.clone() } });
        assert_eq!(crate::settings::definition::tree_of(&draft.drafted()), crate::settings::definition::tree_of(&after));
        // An edit after it lands on the agent's text; one that names a step it removed is skipped.
        draft.push(label("fix", "Repair"));
        assert_eq!(drafted(&draft).0, [("fix".to_string(), "Repair".to_string()), ("tests".to_string(), "Checks".to_string())]);
        draft.push(words("gone", "x"));
        assert_eq!(drafted(&draft).0.len(), 2);
        draft.undo_at(0);
        assert_eq!(drafted(&draft).0, [("fix".to_string(), "Repair".to_string()), ("tests".to_string(), "Tests".to_string())]);
    }

    fn agent(after: &str) -> Edit {
        Edit { author: Author::Agent, said: "asked".into(), change: Change::Definition { before: BASE.into(), after: after.into() } }
    }

    fn renames_tests() -> String {
        BASE.replace("\"label\":\"Tests\"", "\"label\":\"Checks\"")
    }

    fn step(id: &str, label: &str) -> (String, String) {
        (id.to_string(), label.to_string())
    }

    #[test]
    fn an_agents_edit_on_top_of_his_keeps_his() {
        let mut draft = Draft::new(BASE, None);
        draft.push(words("fix", "Make the smallest change."));
        draft.push(agent(&renames_tests()));
        let (steps, words) = drafted(&draft);
        assert_eq!(steps, [step("fix", "Fix"), step("tests", "Checks")]);
        assert_eq!(words, [Some("Make the smallest change.".to_string()), None]);
    }

    #[test]
    fn his_edit_undone_under_an_agents_leaves_the_agents() {
        let mut draft = Draft::new(BASE, None);
        draft.push(words("fix", "Make the smallest change."));
        draft.push(agent(&renames_tests()));
        draft.undo_at(0);
        let (steps, words) = drafted(&draft);
        assert_eq!(steps, [step("fix", "Fix"), step("tests", "Checks")]);
        assert_eq!(words, [Some("Fix it.".to_string()), None]);
    }

    #[test]
    fn the_agents_edit_undone_leaves_only_his() {
        let mut draft = Draft::new(BASE, None);
        draft.push(words("fix", "Make the smallest change."));
        draft.push(agent(&renames_tests()));
        draft.undo_at(1);
        let (steps, words) = drafted(&draft);
        assert_eq!(steps, [step("fix", "Fix"), step("tests", "Tests")]);
        assert_eq!(words, [Some("Make the smallest change.".to_string()), None]);
    }

    #[test]
    fn a_step_the_agent_adds_lands_after_the_one_it_followed() {
        let after = BASE.replace("{\"id\":\"tests\"", "{\"id\":\"review\",\"label\":\"Review\"},{\"id\":\"tests\"");
        let mut draft = Draft::new(BASE, None);
        draft.push(label("fix", "Repair"));
        draft.push(agent(&after));
        assert_eq!(drafted(&draft).0, [step("fix", "Repair"), step("review", "Review"), step("tests", "Tests")]);
    }

    fn swaps_fix_and_tests() -> String {
        BASE.replace(r#"[{"id":"fix","label":"Fix","instructions":"Fix it."},{"id":"tests","label":"Tests"}]"#, r#"[{"id":"tests","label":"Tests"},{"id":"fix","label":"Fix","instructions":"Fix it."}]"#)
    }

    #[test]
    fn steps_the_agent_swaps_are_drawn_swapped_until_its_entry_is_undone() {
        let mut draft = Draft::new(BASE, None);
        draft.push(agent(&swaps_fix_and_tests()));
        assert_eq!(drafted(&draft).0, [step("tests", "Tests"), step("fix", "Fix")]);
        draft.undo_at(0);
        assert_eq!(drafted(&draft).0, [step("fix", "Fix"), step("tests", "Tests")]);
    }

    #[test]
    fn his_words_and_the_agents_move_both_show() {
        let mut draft = Draft::new(BASE, None);
        draft.push(words("fix", "Make the smallest change."));
        draft.push(agent(&swaps_fix_and_tests()));
        let (steps, words) = drafted(&draft);
        assert_eq!(steps, [step("tests", "Tests"), step("fix", "Fix")]);
        assert_eq!(words, [None, Some("Make the smallest change.".to_string())]);
    }

    #[test]
    fn a_step_he_moved_since_the_agent_read_keeps_his_place() {
        let workflow = |order: &str| format!(r#"{{"version":1,"workflow_id":"bug","name":"bug","steps":[{order}]}}"#);
        let read = workflow(r#"{"id":"a"},{"id":"b"},{"id":"c"},{"id":"d"}"#);
        // He swapped c and d after the agent read the draft; the agent swapped a and b.
        let his = workflow(r#"{"id":"a"},{"id":"b"},{"id":"d"},{"id":"c"}"#);
        let agents = workflow(r#"{"id":"b"},{"id":"a"},{"id":"c"},{"id":"d"}"#);
        let mut draft = Draft::new(his, None);
        draft.push(Edit { author: Author::Agent, said: "asked".into(), change: Change::Definition { before: read, after: agents } });
        let order: Vec<String> = drafted(&draft).0.into_iter().map(|(id, _)| id).collect();
        assert_eq!(order, ["b", "a", "d", "c"]);
    }

    fn added(after: &str, id: &str) -> Edit {
        Edit { author: Author::Person, said: format!("Added {id}"), change: Change::AddStep { after: after.into(), id: id.into(), label: "New step".into() } }
    }

    #[test]
    fn a_step_added_stands_after_the_one_it_was_added_after_with_empty_words_and_undo_takes_it_out() {
        let mut draft = Draft::new(BASE, None);
        draft.push(added("fix", "new_step"));
        let (steps, words) = drafted(&draft);
        assert_eq!(steps, [step("fix", "Fix"), step("new_step", "New step"), step("tests", "Tests")]);
        assert_eq!(words, [Some("Fix it.".to_string()), Some(String::new()), None]);
        // His name for it lands on it, and undoing the add leaves the rename with nothing to name: it is skipped.
        draft.push(label("new_step", "Review"));
        assert_eq!(drafted(&draft).0[1], step("new_step", "Review"));
        draft.undo_at(0);
        assert_eq!(drafted(&draft).0, [step("fix", "Fix"), step("tests", "Tests")]);
        draft.undo_last();
        assert_eq!(draft.drafted(), BASE);
    }

    #[test]
    fn a_step_added_after_the_last_ends_the_workflow_and_an_id_taken_is_skipped() {
        let mut draft = Draft::new(BASE, None);
        draft.push(added("tests", "new_step"));
        draft.push(added("fix", "new_step"));
        assert_eq!(drafted(&draft).0, [step("fix", "Fix"), step("tests", "Tests"), step("new_step", "New step")]);
        assert_eq!(draft.edits().len(), 2, "the skipped one stays in the history for its Undo");
    }

    #[test]
    fn a_name_and_words_each_apply_and_each_undo_alone() {
        let mut draft = Draft::new(BASE, None);
        draft.push(label("tests", "Checks"));
        draft.push(words("tests", "Write the test first."));
        assert_eq!(drafted(&draft), (vec![step("fix", "Fix"), step("tests", "Checks")], vec![Some("Fix it.".to_string()), Some("Write the test first.".to_string())]));
        draft.undo_at(0);
        assert_eq!(drafted(&draft), (vec![step("fix", "Fix"), step("tests", "Tests")], vec![Some("Fix it.".to_string()), Some("Write the test first.".to_string())]));
        draft.undo_last();
        assert_eq!(draft.drafted(), BASE);
    }

    #[test]
    fn clearing_takes_a_new_base() {
        let mut draft = Draft::new(BASE, Some(Target::EveryProject));
        draft.push(label("fix", "Repair"));
        draft.clear("{}");
        assert!(draft.is_empty());
        assert_eq!(draft.base(), "{}");
        assert_eq!(draft.target(), Some(&Target::EveryProject));
    }
}
