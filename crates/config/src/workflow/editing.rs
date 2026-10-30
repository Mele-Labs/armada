//! A workflow's steps edited as text, for the copy one Job holds of it. **Text in and text out**: the
//! answer is a definition the loader reads again, which is how an edit that does not fit is found. Comments
//! in the file are not kept, since a copy is read by Fleet and not by a person.

use std::fmt;

use serde_yaml_ng::{Mapping, Value};

/// One edit to the steps of a definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepEdit {
    Add { after: Option<String>, step: NewStep },
    Move { step: String, after: Option<String> },
    Reword { step: String, label: Option<String>, about: Option<String>, instructions: Option<String> },
    Drop { step: String },
}

/// A step as it is added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewStep {
    pub id: Option<String>,
    pub label: String,
    pub about: Option<String>,
    pub instructions: Option<String>,
    pub ends_on: Vec<String>,
}

/// The text after the edit, and the id of the step it was about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edited {
    pub text: String,
    pub step: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditFault {
    NotAWorkflow(String),
    NoSuchStep(String),
    StepExists(String),
    Blank(&'static str),
    Delivers(String),
}

impl fmt::Display for EditFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditFault::NotAWorkflow(why) => write!(f, "the copy is not a workflow: {why}"),
            EditFault::NoSuchStep(step) => write!(f, "the workflow has no step `{step}`"),
            EditFault::StepExists(step) => write!(f, "the workflow already has a step `{step}`"),
            EditFault::Blank(what) => write!(f, "the {what} is blank"),
            EditFault::Delivers(step) => write!(f, "`{step}` delivers the work, and a workflow keeps the step that does"),
        }
    }
}

/// `text` with `edit` made to its steps. A step is found by the id it declares.
pub fn edit_steps(text: &str, edit: &StepEdit) -> Result<Edited, EditFault> {
    let mut root: Value = serde_yaml_ng::from_str(text).map_err(|why| EditFault::NotAWorkflow(why.to_string()))?;
    let steps = root
        .get_mut("steps")
        .and_then(Value::as_sequence_mut)
        .ok_or_else(|| EditFault::NotAWorkflow("it has no steps".into()))?;
    let step = match edit {
        StepEdit::Add { after, step } => {
            let (id, value) = added(step, steps)?;
            let at = place(steps, after.as_deref())?;
            steps.insert(at, value);
            id
        }
        StepEdit::Move { step, after } => {
            let from = position(steps, step)?;
            let moved = steps.remove(from);
            let at = if after.as_deref() == Some(step.as_str()) { from } else { place(steps, after.as_deref())? };
            steps.insert(at, moved);
            step.clone()
        }
        StepEdit::Reword { step, label, about, instructions } => {
            let at = position(steps, step)?;
            let mut changed = false;
            for (key, words) in [("label", label), ("about", about), ("instructions", instructions)] {
                let Some(words) = words.as_deref().map(str::trim).filter(|words| !words.is_empty()) else { continue };
                if let Some(table) = steps[at].as_mapping_mut() {
                    table.insert(key.into(), words.into());
                    changed = true;
                }
            }
            if !changed {
                return Err(EditFault::Blank("words"));
            }
            step.clone()
        }
        StepEdit::Drop { step } => {
            let at = position(steps, step)?;
            if steps[at].get("delivers").and_then(Value::as_bool) == Some(true) {
                return Err(EditFault::Delivers(step.clone()));
            }
            steps.remove(at);
            step.clone()
        }
    };
    let text = serde_yaml_ng::to_string(&root).map_err(|why| EditFault::NotAWorkflow(why.to_string()))?;
    Ok(Edited { text, step })
}

fn id_of(step: &Value) -> Option<&str> {
    step.get("id").and_then(Value::as_str)
}

fn position(steps: &[Value], id: &str) -> Result<usize, EditFault> {
    steps.iter().position(|step| id_of(step) == Some(id)).ok_or_else(|| EditFault::NoSuchStep(id.to_string()))
}

/// Where a step goes that comes after `after`, or first where it comes after nothing.
fn place(steps: &[Value], after: Option<&str>) -> Result<usize, EditFault> {
    after.map_or(Ok(0), |after| position(steps, after).map(|at| at + 1))
}

/// An id a label can stand under: letters and digits, the rest run together as `_`.
fn slug(label: &str) -> String {
    let words: Vec<String> = label
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    words.join("_")
}

fn added(step: &NewStep, steps: &[Value]) -> Result<(String, Value), EditFault> {
    let label = step.label.trim();
    if label.is_empty() {
        return Err(EditFault::Blank("label"));
    }
    let id = step.id.as_deref().map(str::trim).filter(|id| !id.is_empty()).map_or_else(|| slug(label), str::to_string);
    if id.is_empty() {
        return Err(EditFault::Blank("id"));
    }
    if steps.iter().any(|one| id_of(one) == Some(id.as_str())) {
        return Err(EditFault::StepExists(id));
    }
    let mut table = Mapping::new();
    table.insert("id".into(), id.as_str().into());
    table.insert("label".into(), label.into());
    for (key, words) in [("about", &step.about), ("instructions", &step.instructions)] {
        if let Some(words) = words.as_deref().map(str::trim).filter(|words| !words.is_empty()) {
            table.insert(key.into(), words.into());
        }
    }
    if !step.ends_on.is_empty() {
        table.insert("ends_on".into(), Value::Sequence(step.ends_on.iter().map(|end| end.as_str().into()).collect()));
    }
    table.insert("delivers".into(), false.into());
    table.insert("advance_gate".into(), "auto".into());
    Ok((id, Value::Mapping(table)))
}
