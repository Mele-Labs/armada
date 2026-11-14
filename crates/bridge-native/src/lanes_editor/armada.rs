//! What Armada changed in a workflow since a copy of it began, for the owner to choose from. Three pure steps: the
//! differences between what the copy began from (`base`) and Armada's carried text now, a chosen few of them laid onto the
//! draft's text as deltas so what he has edited stays, and the line the change is summed up in.
//!
//! A difference is one of five things a step can have had happen: it was added after another, taken out, moved, named
//! otherwise or given other words. Each names its step by id and by the label a person reads.

use crate::settings::definition::{text_of, tree_of, with_delta, with_label, with_words, Json};

/// What happened to one step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Armada has the step, and the copy's base had not: it follows `after`, or comes first.
    Added { after: Option<String> },
    /// The base had the step and Armada's has not.
    Removed,
    /// The step is in both and stands elsewhere: it follows `after` in Armada's, or comes first.
    Moved { after: Option<String> },
    /// The step's label is another; `was` is the base's.
    Renamed { was: String },
    /// The step's words are other.
    Words,
}

/// One thing Armada changed, on one step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    pub step: String,
    /// The label a person reads: Armada's, and the base's for a step Armada took out.
    pub label: String,
    pub kind: Kind,
}

/// The steps of a definition, where it is a workflow.
fn steps_of(text: &str) -> Option<Vec<Json>> {
    match tree_of(text)?.get("steps")? {
        Json::List(steps) => Some(steps.clone()),
        _ => None,
    }
}

fn id_of(step: &Json) -> Option<&str> {
    step.get("id").and_then(Json::text)
}

/// The label of a step, or its id where it has none.
fn label_of(step: &Json) -> String {
    step.get("label").and_then(Json::text).or_else(|| id_of(step)).unwrap_or_default().to_string()
}

fn words_of(step: &Json) -> Option<&str> {
    step.get("instructions").and_then(Json::text)
}

fn place_of(steps: &[Json], id: &str) -> Option<usize> {
    steps.iter().position(|one| id_of(one) == Some(id))
}

/// The ids of the longest run of `a` that `b` keeps in the same order.
fn kept_in_order<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    let mut longest = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            longest[i][j] = if a[i] == b[j] { longest[i + 1][j + 1] + 1 } else { longest[i + 1][j].max(longest[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut kept) = (0, 0, Vec::new());
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            kept.push(a[i]);
            i += 1;
            j += 1;
        } else if longest[i + 1][j] >= longest[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    kept
}

/// What Armada changed between `base`, the text the copy began from, and `armada`, its carried text now: each step in
/// Armada's order with what happened to it, then each step it took out. None where either is not a workflow.
pub fn differences(base: &str, armada: &str) -> Vec<Difference> {
    let (Some(was), Some(now)) = (steps_of(base), steps_of(armada)) else { return Vec::new() };
    let was_ids: Vec<&str> = was.iter().filter_map(id_of).collect();
    let now_ids: Vec<&str> = now.iter().filter_map(id_of).collect();
    let common_was: Vec<&str> = was_ids.iter().copied().filter(|id| now_ids.contains(id)).collect();
    let common_now: Vec<&str> = now_ids.iter().copied().filter(|id| was_ids.contains(id)).collect();
    let in_order = kept_in_order(&common_was, &common_now);
    let mut out = Vec::new();
    for (at, step) in now.iter().enumerate() {
        let Some(id) = id_of(step) else { continue };
        let after = at.checked_sub(1).and_then(|prev| id_of(&now[prev])).map(String::from);
        let Some(old) = place_of(&was, id).map(|held| &was[held]) else {
            out.push(Difference { step: id.into(), label: label_of(step), kind: Kind::Added { after } });
            continue;
        };
        if !in_order.contains(&id) {
            out.push(Difference { step: id.into(), label: label_of(step), kind: Kind::Moved { after } });
        }
        if label_of(old) != label_of(step) {
            out.push(Difference { step: id.into(), label: label_of(step), kind: Kind::Renamed { was: label_of(old) } });
        }
        if words_of(old) != words_of(step) {
            out.push(Difference { step: id.into(), label: label_of(step), kind: Kind::Words });
        }
    }
    for step in &was {
        if let Some(id) = id_of(step).filter(|id| !now_ids.contains(id)) {
            out.push(Difference { step: id.into(), label: label_of(step), kind: Kind::Removed });
        }
    }
    out
}

/// `text`, a workflow's steps, as a tree to change in place.
fn reshaped(text: &str, change: impl FnOnce(&mut Vec<Json>) -> Option<()>) -> Option<String> {
    let mut tree = tree_of(text)?;
    let Json::Map(fields) = &mut tree else { return None };
    let (_, Json::List(steps)) = fields.iter_mut().find(|(name, _)| name == "steps")? else { return None };
    change(steps)?;
    Some(text_of(&tree))
}

/// Just after the last of `preceding` that `steps` holds, or first where it holds none.
fn place_after(steps: &[Json], preceding: &[Json]) -> usize {
    preceding.iter().rev().filter_map(id_of).find_map(|prev| place_of(steps, prev)).map_or(0, |prev| prev + 1)
}

/// `base` with the `chosen` differences made and nothing else of Armada's.
fn with_taken(base: &str, armada: &[Json], chosen: &[Difference]) -> String {
    let mut text = base.to_string();
    for one in chosen {
        let Some(now) = place_of(armada, &one.step) else {
            // Only a step Armada took out is not in its text.
            if one.kind == Kind::Removed {
                text = reshaped(&text, |steps| {
                    steps.retain(|held| id_of(held) != Some(one.step.as_str()));
                    Some(())
                })
                .unwrap_or(text);
            }
            continue;
        };
        let next = match &one.kind {
            Kind::Words => with_words(&text, &one.step, words_of(&armada[now]).unwrap_or_default()),
            Kind::Renamed { .. } => with_label(&text, &one.step, &label_of(&armada[now])),
            Kind::Added { .. } => reshaped(&text, |steps| {
                if place_of(steps, &one.step).is_some() {
                    return None;
                }
                let at = place_after(steps, &armada[..now]);
                steps.insert(at, armada[now].clone());
                Some(())
            }),
            Kind::Moved { .. } => reshaped(&text, |steps| {
                let held = steps.remove(place_of(steps, &one.step)?);
                let at = place_after(steps, &armada[..now]);
                steps.insert(at, held);
                Some(())
            }),
            Kind::Removed => None,
        };
        if let Some(next) = next {
            text = next;
        }
    }
    text
}

/// The draft's `current` text with the `chosen` differences laid onto it, and what he has edited left as it is: each is
/// carried in as a delta from `base`, so a step he moved, renamed or reworded and Armada did not touch stays, and a step
/// of the chosen set that he has taken out stays out. None where any of the three is not a workflow.
pub fn take(current: &str, base: &str, armada: &str, chosen: &[Difference]) -> Option<String> {
    let (Some(_), Some(_), Some(steps)) = (steps_of(current), steps_of(base), steps_of(armada)) else { return None };
    if chosen.is_empty() {
        return Some(current.to_string());
    }
    with_delta(current, base, &with_taken(base, &steps, chosen))
}

/// The change summary for what was taken, naming each and never counting them: `Took from Armada: Review's words, Notes
/// added`. None where nothing was.
pub fn took(chosen: &[Difference]) -> Option<String> {
    let named: Vec<String> = chosen
        .iter()
        .map(|one| match &one.kind {
            Kind::Added { .. } => format!("{} added", one.label),
            Kind::Removed => format!("{} removed", one.label),
            Kind::Moved { .. } => format!("{} moved", one.label),
            Kind::Renamed { .. } => format!("{}'s name", one.label),
            Kind::Words => format!("{}'s words", one.label),
        })
        .collect();
    (!named.is_empty()).then(|| format!("Took from Armada: {}", named.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::definition::{steps_of as listed, words_of as held_words};

    /// A workflow of `(id, label, words)` steps.
    fn workflow(steps: &[(&str, &str, &str)]) -> String {
        let steps: Vec<String> = steps.iter().map(|(id, label, words)| format!(r#"{{"id":"{id}","label":"{label}","instructions":"{words}"}}"#)).collect();
        format!(r#"{{"version":1,"workflow_id":"feature","name":"feature","steps":[{}]}}"#, steps.join(","))
    }

    /// What the copy began from, and Armada's text now: Plan reworded, Review added after it, Test named Check, Build
    /// moved to the end and Notes taken out.
    fn base() -> String {
        workflow(&[("plan", "Plan", "Plan it."), ("build", "Build", "Build it."), ("test", "Test", "Test it."), ("land", "Land", "Land it."), ("notes", "Notes", "Write notes.")])
    }

    fn armada() -> String {
        workflow(&[("plan", "Plan", "Plan small."), ("review", "Review", "Review it."), ("test", "Check", "Test it."), ("land", "Land", "Land it."), ("build", "Build", "Build it.")])
    }

    fn named(step: &str, label: &str, kind: Kind) -> Difference {
        Difference { step: step.into(), label: label.into(), kind }
    }

    #[test]
    fn each_thing_armada_changed_names_its_step_by_id_and_label() {
        let found = differences(&base(), &armada());
        assert_eq!(
            found,
            [
                named("plan", "Plan", Kind::Words),
                named("review", "Review", Kind::Added { after: Some("plan".into()) }),
                named("test", "Check", Kind::Renamed { was: "Test".into() }),
                named("build", "Build", Kind::Moved { after: Some("land".into()) }),
                named("notes", "Notes", Kind::Removed),
            ]
        );
    }

    #[test]
    fn a_step_added_first_follows_nothing_and_the_same_text_or_a_non_workflow_differs_in_nothing() {
        let first = workflow(&[("intake", "Intake", "Take it in."), ("plan", "Plan", "Plan it.")]);
        let was = workflow(&[("plan", "Plan", "Plan it.")]);
        assert_eq!(differences(&was, &first), [named("intake", "Intake", Kind::Added { after: None })]);
        assert!(differences(&base(), &base()).is_empty());
        assert!(differences("not json", &base()).is_empty() && differences(&base(), "steps: []").is_empty());
    }

    #[test]
    fn a_step_that_both_changes_its_name_and_its_words_is_two_differences() {
        let was = workflow(&[("plan", "Plan", "Plan it.")]);
        let now = workflow(&[("plan", "Scope", "Scope it.")]);
        assert_eq!(differences(&was, &now), [named("plan", "Scope", Kind::Renamed { was: "Plan".into() }), named("plan", "Scope", Kind::Words)]);
    }

    /// His own edits to the draft: Land renamed Ship, and Build's words his own.
    fn his() -> String {
        let renamed = with_label(&base(), "land", "Ship").expect("land is a step");
        with_words(&renamed, "build", "Build mine.").expect("build is a step")
    }

    #[test]
    fn what_is_chosen_is_laid_onto_the_draft_and_what_he_edited_and_armada_did_not_touch_stays() {
        let all = differences(&base(), &armada());
        let chosen: Vec<Difference> = all.iter().filter(|one| matches!(one.kind, Kind::Words | Kind::Added { .. } | Kind::Removed)).cloned().collect();
        let taken = take(&his(), &base(), &armada(), &chosen).expect("all three are workflows");
        assert_eq!(listed(&taken), [("plan".into(), "Plan".into()), ("review".into(), "Review".into()), ("build".into(), "Build".into()), ("test".into(), "Test".into()), ("land".into(), "Ship".into())]);
        assert_eq!(held_words(&taken, "plan").as_deref(), Some("Plan small."), "the words he chose");
        assert_eq!(held_words(&taken, "review").as_deref(), Some("Review it."), "an added step comes with its words");
        assert_eq!(held_words(&taken, "build").as_deref(), Some("Build mine."), "his words on a step Armada did not reword");
    }

    #[test]
    fn a_move_and_a_rename_are_taken_when_chosen_and_not_otherwise() {
        let all = differences(&base(), &armada());
        let moved: Vec<Difference> = all.iter().filter(|one| matches!(one.kind, Kind::Moved { .. })).cloned().collect();
        let taken = take(&base(), &base(), &armada(), &moved).expect("workflows");
        assert_eq!(listed(&taken).iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), ["plan", "test", "land", "build", "notes"]);
        assert_eq!(held_words(&taken, "plan").as_deref(), Some("Plan it."), "only the move");
        let renamed: Vec<Difference> = all.iter().filter(|one| matches!(one.kind, Kind::Renamed { .. })).cloned().collect();
        let taken = take(&base(), &base(), &armada(), &renamed).expect("workflows");
        assert_eq!(listed(&taken)[2], ("test".to_string(), "Check".to_string()));
    }

    #[test]
    fn nothing_chosen_leaves_the_draft_and_a_step_he_took_out_stays_out() {
        assert_eq!(take(&his(), &base(), &armada(), &[]).as_deref(), Some(his().as_str()));
        let without = workflow(&[("build", "Build", "Build it.")]);
        let reworded = [named("plan", "Plan", Kind::Words)];
        let taken = take(&without, &base(), &armada(), &reworded).expect("workflows");
        assert_eq!(listed(&taken), [("build".to_string(), "Build".to_string())]);
        assert_eq!(take("not json", &base(), &armada(), &reworded), None);
    }

    #[test]
    fn the_summary_names_what_was_taken_and_never_counts_it() {
        let all = differences(&base(), &armada());
        let words = all.iter().find(|one| one.kind == Kind::Words).cloned().expect("plan's words");
        let added = all.iter().find(|one| matches!(one.kind, Kind::Added { .. })).cloned().expect("review");
        assert_eq!(took(&[words, added]).as_deref(), Some("Took from Armada: Plan's words, Review added"));
        assert_eq!(took(&all).as_deref(), Some("Took from Armada: Plan's words, Review added, Check's name, Build moved, Notes removed"));
        assert_eq!(took(&[]), None);
    }
}
