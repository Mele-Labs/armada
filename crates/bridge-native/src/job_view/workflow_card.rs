//! A proposed workflow's draft card drawn as the workflow: its name and what it is for, then one
//! lane a step, each with what it does, what ends it and how it delivers, and what the agent is told
//! folded under a toggle. Fleet reads the steps off the draft (`WorkflowChange::after_steps`); a card
//! whose steps have not arrived draws the definition's text, as it did before.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{ChangeStep, SessionRow, ThreadItem, WorkflowChange};

use super::acts::{button, Tone};
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// The draft row's title for a workflow change: `Workflow <id>`. Rules and lifecycles carry no steps.
const WORKFLOW_TITLE: &str = "Workflow ";

/// What a lane's gate glyph draws: the glyph, its tooltip and its ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GateMark {
    pub glyph: &'static str,
    pub named: &'static str,
    pub ink: Token,
}

/// One fact about how a step delivers, drawn as a line in its lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fact {
    pub glyph: &'static str,
    pub said: &'static str,
}

/// One step, as its lane draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Lane {
    pub id: String,
    pub label: String,
    pub about: Option<String>,
    /// `Ends when …`, as Fleet reads the step's end.
    pub ends: Option<String>,
    pub gate: Option<GateMark>,
    pub facts: Vec<Fact>,
    pub instructions: Option<String>,
    pub delivers: bool,
}

/// A proposed workflow, as the card draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Card {
    pub name: String,
    pub id: String,
    pub scope: &'static str,
    pub for_requests: Option<String>,
    pub lanes: Vec<Lane>,
}

/// The glyph and name of a plan gate, as the reverted Electron lanes and a plan's Now draw them.
/// His own gates take his colour; the ones Fleet sees for itself take the kind they close.
pub(super) fn gate_mark(word: &str) -> Option<GateMark> {
    let (glyph, named, ink) = match word {
        "plan_approved" => ("stamp", "Your approval of the plan", token::YOU),
        "walk_approved" => ("footprints", "Your approval of the walk", token::YOU),
        "sketch_approved" => ("pencil-ruler", "Your approval of the sketch", token::YOU),
        "document_approved" => ("notebook-pen", "Your approval of the document", token::YOU),
        "you_approved" => ("circle-check", "Your Approve", token::YOU),
        "checks_green" => ("shield-check", "Checks pass", token::KIND_GATE),
        "merged" => ("git-merge", "The merge", token::KIND_DELIVERY),
        "jobs_ended" => ("link", "The Jobs it started end", token::KIND_GATE),
        _ => return None,
    };
    Some(GateMark { glyph, named, ink })
}

/// The delivery facts of a step, each a line in its lane.
pub(super) fn facts(step: &ChangeStep) -> Vec<Fact> {
    let mut facts = Vec::new();
    if step.asks_delivery {
        facts.push(Fact { glyph: "signpost", said: "Asks how to deliver" });
    }
    if step.delivers {
        facts.push(Fact { glyph: "git-pull-request", said: "Delivers" });
    }
    facts
}

pub(super) fn lane(step: &ChangeStep) -> Lane {
    Lane {
        id: step.id.as_str().to_string(),
        label: step.label.clone(),
        about: step.about.clone(),
        ends: step.ends_when.as_ref().map(|ends| format!("Ends when {ends}")),
        gate: step.gate.as_deref().and_then(gate_mark),
        facts: facts(step),
        instructions: step.instructions.clone(),
        delivers: step.delivers,
    }
}

/// The card for the change Fleet listed, where its steps arrived and it drafts the text the row
/// shows. `None` draws the text: no change read, no steps, or a send back redrafted it since.
pub(super) fn card_of(change: Option<&WorkflowChange>, text: &str) -> Option<Card> {
    let change = change?;
    if change.after.as_deref() != Some(text) {
        return None;
    }
    let steps = change.after_steps.as_ref().filter(|steps| !steps.is_empty())?;
    let id = change.workflow_id.as_str().to_string();
    Some(Card {
        name: change.after_name.clone().unwrap_or_else(|| id.clone()),
        id,
        scope: if change.source == "kit" { "Every repository on this Mac" } else { "This repository" },
        for_requests: change.after_for_requests.clone(),
        lanes: steps.iter().map(lane).collect(),
    })
}

/// The open workflow drafts of a thread whose change is to be read: change id, workflow id, text.
pub(super) fn wanted(items: &[ThreadItem]) -> Vec<(i64, String, String)> {
    items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::Row { row: SessionRow::Draft { title, text, settled: None, change: Some(change), .. } } => {
                title.strip_prefix(WORKFLOW_TITLE).map(|workflow| (*change, workflow.trim().to_string(), text.clone()))
            }
            _ => None,
        })
        .collect()
}

impl JobView {
    /// Ask Fleet for the change behind each open workflow draft on screen. Run as Fleet moves, never in a render.
    pub(super) fn read_workflow_changes(&mut self, cx: &mut Context<Self>) {
        let (Some(id), Some(fleet)) = (self.shown.clone(), self.fleet.clone()) else { return };
        let Some(held) = self.thread(&id, cx) else { return };
        let wanted = wanted(&held.items);
        if wanted.is_empty() {
            return;
        }
        let manifest = self.record(&id, cx).and_then(|record| record.manifest_id.as_ref()).map(|manifest| manifest.as_str().to_string());
        fleet.update(cx, |fleet, cx| {
            for (change, workflow, text) in &wanted {
                fleet.read_workflow_change(*change, workflow, manifest.as_deref(), text, cx);
            }
        });
    }

    /// The draft card's body for a proposed workflow whose steps Fleet read. `None` draws the text.
    pub(super) fn workflow_card(&self, ix: usize, key: &str, change: Option<i64>, text: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let fleet = self.fleet.as_ref()?;
        let card = card_of(change.and_then(|change| fleet.read(cx).held().workflow_changes.get(&change)), text)?;
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_OVERLAY))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .child(glyph(look, "workflow", Size::Mark, look.color(token::KIND_STEP)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(look.length(token::TEXT_BASE))
                    .line_height(look.length(token::LEADING_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(card.name.clone())),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .gap(look.length(token::SPACE_3))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(div().font_family(mono(look)).child(SharedString::from(card.id.clone())))
                    .child(card.scope),
            );
        let for_requests = card.for_requests.clone().map(|asked| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(super::rows::eyebrow(look, "For requests").text_color(look.color(token::FG_MUTED)))
                .child(SharedString::from(asked))
        });
        let last = card.lanes.len().saturating_sub(1);
        let lanes = card.lanes.iter().enumerate().map(|(at, lane)| self.lane(ix, key, at, at == last, lane, look, cx));
        Some(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .gap(look.length(token::SPACE_3))
                .child(head)
                .children(for_requests)
                .child(div().flex().flex_col().min_w_0().children(lanes))
                .into_any_element(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn lane(&self, ix: usize, key: &str, at: usize, last: bool, lane: &Lane, look: &Look, cx: &Context<Self>) -> AnyElement {
        let kind = if lane.delivers { token::KIND_DELIVERY } else { token::KIND_STEP };
        let told_key = format!("{key}:step:{}", lane.id);
        let told = self.open_in_card(&told_key);
        // The rail: the step's number in a ring of its kind, and a hairline down to the next.
        let rail = div()
            .flex()
            .flex_col()
            .items_center()
            .flex_none()
            .w(look.length(token::H_CONTROL))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL))
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(kind))
                    .bg(look.color(token::BG_RAISED))
                    .text_color(look.color(kind))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child(SharedString::from((at + 1).to_string())),
            )
            .when(!last, |rail| rail.child(div().flex_1().w(look.length(token::BORDER_WIDTH) * 2.).bg(look.color(token::BORDER_STRONG))));
        let line = |said: SharedString, ink: Token| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(ink))
                .child(said)
        };
        let ends = lane.ends.clone().map(|ends| {
            let row = div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT));
            match lane.gate {
                Some(gate) => row
                    .child(
                        labelled(
                            div()
                                .id(SharedString::from(format!("gate:{key}:{}", lane.id)))
                                .flex_none()
                                .child(glyph(look, gate.glyph, Size::Small, look.color(gate.ink)))
                                .tooltip(tip(gate.named)),
                            Role::Image,
                            gate.named,
                        ),
                    )
                    .child(SharedString::from(ends)),
                None => row.child(SharedString::from(ends)),
            }
        });
        let facts = lane.facts.iter().map(|fact| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph(look, fact.glyph, Size::Small, look.color(token::KIND_DELIVERY)))
                .child(fact.said)
        });
        let toggle = lane.instructions.as_ref().map(|instructions| {
            let name = format!("What the agent is told on {}", lane.label);
            let flip = told_key.clone();
            div()
                .flex()
                .flex_col()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    button(look, SharedString::from(format!("told:{told_key}")), "What the agent is told", Tone::Ghost, false, None, true)
                        .on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, &flip, cx))),
                    Role::Button,
                    name,
                ))
                .children(told.then(|| {
                    div()
                        .w_full()
                        .min_w_0()
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(look.color(token::BG_BASE))
                        .child(self.prose(format!("{told_key}:text"), instructions, look))
                }))
        });
        let body = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(look.length(token::SPACE_1))
            .mb(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_SUNKEN))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                div()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(lane.label.clone())),
            )
            .children(lane.about.clone().map(|about| line(SharedString::from(about), token::FG_MUTED)))
            .children(ends)
            .children(facts)
            .children(toggle);
        labelled(
            div().id(SharedString::from(format!("lane:{key}:{}", lane.id))).flex().gap(look.length(token::SPACE_3)).min_w_0().child(rail).child(body),
            Role::Group,
            lane.label.clone(),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{ChangeOrigin, ChangeState, DraftHow, DraftSettled, Instant, StepId, WorkflowId};

    fn step(id: &str) -> ChangeStep {
        ChangeStep {
            id: StepId::carried(id),
            label: format!("Step {id}"),
            instructions: None,
            ends_when: None,
            ends_any: Vec::new(),
            passed_over_when: None,
            about: None,
            gate: None,
            asks_delivery: false,
            delivers: false,
        }
    }

    fn change(steps: Option<Vec<ChangeStep>>) -> WorkflowChange {
        WorkflowChange {
            id: 7,
            workflow_id: WorkflowId::carried("annotations"),
            source: String::from("repository"),
            step_id: None,
            origin: ChangeOrigin::New,
            before: String::new(),
            after: Some(String::from("{\"name\":\"annotations\"}")),
            state: ChangeState::Ready,
            reason: None,
            missing: Vec::new(),
            before_steps: None,
            after_steps: steps,
            after_name: Some(String::from("Work the notes")),
            after_for_requests: Some(String::from("Notes he left on Bridge")),
            notes: None,
            at: Instant::carried("2026-11-12T09:00:00.000Z"),
        }
    }

    const TEXT: &str = "{\"name\":\"annotations\"}";

    #[test]
    fn workflow_card_reads_each_step_into_a_lane() {
        let mut read = step("read");
        read.about = Some(String::from("Reads each open note."));
        read.ends_when = Some(String::from("you approve"));
        read.gate = Some(String::from("you_approved"));
        read.instructions = Some(String::from("Open every note."));
        let card = card_of(Some(&change(Some(vec![read, step("fix")]))), TEXT).expect("a card");
        assert_eq!(card.name, "Work the notes");
        assert_eq!(card.id, "annotations");
        assert_eq!(card.scope, "This repository");
        assert_eq!(card.for_requests.as_deref(), Some("Notes he left on Bridge"));
        assert_eq!(card.lanes.len(), 2);
        let first = &card.lanes[0];
        assert_eq!(first.label, "Step read");
        assert_eq!(first.about.as_deref(), Some("Reads each open note."));
        assert_eq!(first.ends.as_deref(), Some("Ends when you approve"));
        assert_eq!(first.gate.map(|gate| (gate.glyph, gate.named)), Some(("circle-check", "Your Approve")));
        assert_eq!(first.instructions.as_deref(), Some("Open every note."));
        assert_eq!(card.lanes[1].ends, None);
        assert_eq!(card.lanes[1].gate, None);
    }

    #[test]
    fn workflow_card_falls_back_to_the_text_without_steps() {
        assert_eq!(card_of(None, TEXT), None);
        assert_eq!(card_of(Some(&change(None)), TEXT), None);
        assert_eq!(card_of(Some(&change(Some(Vec::new()))), TEXT), None);
        // A send back redrafted it since Fleet was read: the row's text stands.
        assert_eq!(card_of(Some(&change(Some(vec![step("read")]))), "{\"name\":\"other\"}"), None);
    }

    #[test]
    fn workflow_card_says_delivery_as_facts_in_the_lane() {
        let mut ship = step("ship");
        ship.asks_delivery = true;
        ship.delivers = true;
        let lane = lane(&ship);
        assert_eq!(lane.facts.iter().map(|fact| fact.said).collect::<Vec<_>>(), vec!["Asks how to deliver", "Delivers"]);
        assert!(lane.delivers);
        assert!(super::lane(&step("read")).facts.is_empty());
    }

    #[test]
    fn workflow_card_reads_only_open_workflow_drafts() {
        let draft = |id: &str, title: &str, change: Option<i64>, settled: Option<DraftSettled>| ThreadItem::Row {
            row: SessionRow::Draft {
                id: id.to_string(),
                at: Instant::carried("2026-11-12T09:00:00.000Z"),
                title: title.to_string(),
                text: TEXT.to_string(),
                before: None,
                settled,
                change,
            },
        };
        let items = vec![
            draft("a", "Workflow annotations", Some(7), None),
            draft("b", "Rule tidy", Some(8), None),
            draft("c", "README", None, None),
            draft("d", "Workflow done", Some(9), Some(DraftSettled { how: DraftHow::Approved, words: None, at: Instant::carried("2026-11-12T09:01:00.000Z") })),
        ];
        assert_eq!(wanted(&items), vec![(7, String::from("annotations"), TEXT.to_string())]);
    }
}
