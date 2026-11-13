//! The draft: a workflow's definition as it was read (`base`) and every edit since, in order. What the canvas
//! shows and what Publish writes is the base with each edit replayed, never a copy that is edited in place, so
//! dropping any one edit is dropping it from the list and replaying the rest.

use crate::settings::definition::{with_label, with_words};

/// Whose edit it is: the person's own, or an agent's that was asked for in the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Author {
    Person,
    Agent,
}

/// What an edit does to the definition's text. Later changes (a step's fields, a gap's new step, a rule) are more
/// variants here and a line in `apply`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// The whole definition replaced: what an agent's change is. `before` is the text it was drafted from; the
    /// change takes `after` whole, so an edit made before it on the same text is not carried into it.
    Definition { before: String, after: String },
    /// One step's words.
    StepWords { step: String, words: String },
    /// One step's name.
    StepLabel { step: String, label: String },
}

impl Change {
    /// The text after the change, or none where it does not fit the text (a step it names is not there).
    pub fn apply(&self, text: &str) -> Option<String> {
        match self {
            Change::Definition { after, .. } => Some(after.clone()),
            Change::StepWords { step, words } => with_words(text, step, words),
            Change::StepLabel { step, label } => with_label(text, step, label),
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
    fn an_agents_whole_definition_takes_the_place_of_what_it_was_drafted_from() {
        let after = BASE.replace("\"label\":\"Tests\"", "\"label\":\"Checks\"");
        let mut draft = Draft::new(BASE, None);
        draft.push(Edit { author: Author::Agent, said: "Rename Tests".into(), change: Change::Definition { before: BASE.into(), after: after.clone() } });
        assert_eq!(draft.drafted(), after);
        // An edit after it lands on the agent's text; one that names a step it removed is skipped.
        draft.push(label("fix", "Repair"));
        assert_eq!(drafted(&draft).0, [("fix".to_string(), "Repair".to_string()), ("tests".to_string(), "Checks".to_string())]);
        draft.push(words("gone", "x"));
        assert_eq!(drafted(&draft).0.len(), 2);
        draft.undo_at(0);
        assert_eq!(drafted(&draft).0, [("fix".to_string(), "Repair".to_string()), ("tests".to_string(), "Tests".to_string())]);
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
