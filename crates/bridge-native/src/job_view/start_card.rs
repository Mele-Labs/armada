//! A held Job's start card, as Armada Classic draws it (`StartPicks.tsx`, `sv-start-strip`): the Job opens before any agent
//! moves, its first message is the box, a strip over the box settles its Workflow and its Delivery, and Send reads Start.
//! A press begins the agent (`begin_session`). Nothing here once the Job has started.
//!
//! Neither pick is ever blank once the proposer has read: each carries a default, and a mark only where the proposer or his
//! usual chose it. Start pressed while the request is still read is queued, and Fleet starts it when the reading ends.

use std::collections::{HashMap, HashSet};

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Focusable, Role, SharedString, Window};
use ipc::{BeginSession, DeliveryChoice, DeliveryPickedFrom, SessionId, SessionRecord, StartHeld};

use super::composer::Kept;
use super::head::drawn;
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What a person has settled on the cards of held Jobs that Fleet has not yet answered.
#[derive(Default)]
pub(super) struct Starts {
    /// The landing he picked, by Job, until the Job has begun.
    delivered: HashMap<SharedString, DeliveryChoice>,
    /// The Jobs whose Workflow he set to None: Fleet reads a custom Job's workflow as none, which is also how a Job it has not read looks.
    custom: HashSet<SharedString>,
    /// Start was pressed and Fleet has not answered.
    starting: HashSet<SharedString>,
    /// The Jobs whose box took the first message once.
    seeded: HashSet<SharedString>,
}

/// A held Job's card, read off its record.
pub(super) struct Card {
    start: StartHeld,
    /// The proposer is reading the request, so the picks wait.
    reading: bool,
    /// Workflow and Delivery are the proposer's own picks, not touched since.
    proposed_workflow: bool,
    /// The workflow the Job runs under, by id, and the ones its repository offers.
    kind: Option<String>,
    kind_name: Option<String>,
    kinds: Vec<(String, String)>,
    /// Workflow is None by his word.
    none: bool,
    /// How the work lands, asked of the Job where its workflow has a step that asks.
    lands: bool,
    landing: Option<DeliveryChoice>,
    /// Where the landing came from: his words or his usual, with what the mark says.
    standing: Option<(&'static str, String)>,
    starting: bool,
}

impl Card {
    /// Fleet gave the Job a first message, so an empty box has nothing to start with.
    pub(super) fn has_message(&self) -> bool {
        self.start.message.is_some()
    }

    pub(super) fn starting(&self) -> bool {
        self.starting
    }
}

const PICKS: [&str; 3] = ["auto_land", "open_pr", "keep_local"];

/// A workflow's name made plural for "Your usual for features in armada".
fn plural(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.ends_with('s') || lower.ends_with('x') || lower.ends_with("ch") || lower.ends_with("sh") {
        format!("{lower}es")
    } else {
        format!("{lower}s")
    }
}

impl JobView {
    /// The card a held Job stands on, or None for a Job that is not held.
    pub(super) fn held_card(&self, id: &str, cx: &gpui::App) -> Option<Card> {
        let job = self.record(id, cx)?;
        if job.state == ipc::SessionState::Ended {
            return None;
        }
        let start = job.hosted.as_ref()?.start.clone()?;
        Some(self.card_of(job, start, id, cx))
    }

    fn card_of(&self, job: &SessionRecord, start: StartHeld, id: &str, cx: &gpui::App) -> Card {
        let work = self.work(job, cx);
        let reading = job.proposal.as_ref().is_some_and(|proposal| proposal.reading);
        let none = self.starts.custom.contains(id);
        let kinds = work.choices.clone();
        // Fleet gives the Job its workflow once it has read; `feature`, else the first, is for a Fleet that has not.
        let kind = if reading || none {
            None
        } else {
            work.chosen.clone().or_else(|| kinds.iter().find(|(one, _)| one == "feature").or(kinds.first()).map(|(one, _)| one.clone()))
        };
        // The name of the workflow the Job runs under: its repository's list, else the name its record carries.
        let kind_name = kind.as_ref().and_then(|kind| kinds.iter().find(|(one, _)| one == kind)).map(|(_, name)| name.clone()).or_else(|| kind.as_ref().filter(|kind| work.chosen.as_ref() == Some(*kind)).and(work.name.clone()));
        let lands = start.asks_delivery && !none;
        // `start.picked` is Fleet's landing; Auto land is only for a Fleet that sends none.
        let landing = if reading {
            None
        } else {
            Some(self.starts.delivered.get(id).copied().or_else(|| start.picked.as_ref().map(|picked| picked.choice)).unwrap_or(DeliveryChoice::AutoLand))
        };
        // Marked while in use, only where Fleet took it from his usual or his words; its own default is no choice of his.
        let from = start.picked.as_ref().filter(|picked| landing == Some(picked.choice)).map(|picked| picked.from);
        let standing = match from {
            Some(DeliveryPickedFrom::Named) => Some(("quote", "Named in your request".to_string())),
            Some(DeliveryPickedFrom::Usual) => {
                let said = match &kind_name {
                    Some(name) => format!("Your usual for {} in {}", plural(name), job.manifest_id.as_ref().map_or_else(|| job.cwd.clone(), |manifest| manifest.as_str().to_string())),
                    None => "Your usual".to_string(),
                };
                Some(("repeat", said))
            }
            _ => None,
        };
        Card {
            reading,
            proposed_workflow: kind.is_some() && kind == work.chosen && work.proposed,
            kind,
            kind_name,
            kinds,
            none,
            lands,
            landing,
            standing,
            starting: start.queued || self.starts.starting.contains(id),
            start,
        }
    }

    /// The first message a held Job will begin with is the box's first draft, once.
    pub(super) fn seed_start(&mut self, id: &SharedString, kept: Kept, cx: &gpui::App) -> Kept {
        if !kept.text.is_empty() || self.starts.seeded.contains(id) {
            return kept;
        }
        let Some(message) = self.record(id, cx).and_then(|job| job.hosted.as_ref()).and_then(|hosted| hosted.start.as_ref()).and_then(|start| start.message.clone()) else {
            return kept;
        };
        self.starts.seeded.insert(id.clone());
        Kept { text: message.trim_end_matches('\n').to_string(), tags: Vec::new() }
    }

    /// The strip over the box: Workflow and Delivery, the proposer's marks, and a ring while the start waits on the reading.
    pub(super) fn start_strip(&self, card: &Card, look: &Look, cx: &Context<Self>) -> AnyElement {
        let ink = look.color(token::ACCENT);
        let mark = |id: &'static str, icon: &'static str, said: SharedString| {
            labelled(div().id(id).flex_none().flex().items_center().tooltip(tip(said.clone())).child(glyph(look, icon, Size::Small, ink)), Role::Image, said)
        };
        let working = card.starting.then(|| {
            labelled(
                div().id("start-starting").flex_none().flex().items_center().tooltip(tip("Starting")).child(crate::titlebar::parts::spinner("start-ring", look.length(token::SPACE_3), ink, cx)),
                Role::Image,
                "Starting",
            )
        });
        let reading = card.reading.then(|| mark("start-reading", "text-cursor", "Reading the request".into()));
        let workflow_said = if card.none {
            "None".to_string()
        } else {
            card.kind_name.clone().unwrap_or_else(|| "Workflow".to_string())
        };
        let workflow = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .child(self.strip_pick("start-workflow", "Workflow", workflow_said, "Workflow".into(), card.reading, look, cx, |this, window, cx| this.open_workflow_pick(window, cx)))
            .children(card.proposed_workflow.then(|| mark("start-workflow-proposed", "lightbulb", "Picked by the proposer".into())));
        let delivery = card.lands.then(|| {
            let landing = card.landing;
            let said = landing.map_or("Select delivery", |one| one.label());
            let detail = landing.map_or("Delivery", |one| one.detail());
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .child(self.strip_pick("start-delivery", "Delivery", said.to_string(), detail.into(), card.reading, look, cx, |this, window, cx| this.open_delivery_pick(window, cx)))
                .children(card.standing.clone().map(|(icon, said)| mark("start-delivery-mark", icon, said.into())))
        });
        div()
            .id("start-strip")
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .rounded_t(look.length(token::RADIUS_MD))
            .bg(look.color(token::ACCENT_MUTED))
            .children(working)
            .children(reading)
            .child(workflow)
            .children(delivery)
            .into_any_element()
    }

    /// One of the strip's selects: a sunken well in a strong edge, as the permission picker is.
    #[allow(clippy::too_many_arguments)]
    fn strip_pick(
        &self,
        id: &'static str,
        name: &'static str,
        said: String,
        tooltip: SharedString,
        off: bool,
        look: &Look,
        cx: &Context<Self>,
        open: impl Fn(&mut JobView, &mut Window, &mut Context<JobView>) + 'static,
    ) -> AnyElement {
        labelled(
            div()
                .id(id)
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL_SM))
                .pl(look.length(token::SPACE_3))
                .pr(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_STRONG))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                .when(off, |pick| pick.opacity(0.5))
                .when(!off, |pick| {
                    pick.cursor_pointer()
                        .text_color(look.color(token::FG_DEFAULT))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .on_click(cx.listener(move |this, _, window, cx| open(this, window, cx)))
                })
                .when(off, |pick| pick.text_color(look.color(token::FG_MUTED)))
                .tooltip(tip(tooltip))
                .child(SharedString::from(said))
                .child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_SUBTLE))),
            Role::ComboBox,
            name,
        )
        .into_any_element()
    }

    /// The Workflow list under its select: None, then each workflow the Job's repository holds, the Job's ticked.
    fn open_workflow_pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(card) = self.held_card(&id, cx) else { return };
        if card.reading {
            return;
        }
        let ids: Vec<String> = card.kinds.iter().map(|(one, _)| one.clone()).collect();
        let mut entries = vec![Entry::Choice { id: "none", label: "None".into(), on: card.none }];
        entries.extend(card.kinds.iter().take(super::workflow::PICKS.len()).enumerate().map(|(at, (one, name))| Entry::Choice {
            id: super::workflow::PICKS[at],
            label: name.clone().into(),
            on: !card.none && card.kind.as_deref() == Some(one.as_str()),
        }));
        let at = drawn(window, cx, Role::ComboBox, "Workflow").map_or_else(|| window.mouse_position(), super::head::below_left);
        let back = Some(self.composer.input.read(cx).focus_handle(cx));
        self.open_button_menu(
            "Workflow",
            entries,
            at,
            back,
            move |this, picked, window, cx| {
                let pick = super::workflow::PICKS.iter().position(|one| *one == picked).and_then(|at| ids.get(at).cloned());
                this.pick_start_workflow(pick, window, cx);
            },
            window,
            cx,
        );
    }

    /// None is a custom Job: Fleet is told at once, as for any other workflow, and no landing is asked of it.
    fn pick_start_workflow(&mut self, pick: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if pick.is_none() {
            self.starts.custom.insert(id.clone());
        } else {
            self.starts.custom.remove(&id);
        }
        self.choose_workflow(pick, window, cx);
    }

    /// The Delivery list under its select: the three landings Fleet offers, the Job's ticked.
    fn open_delivery_pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(card) = self.held_card(&id, cx) else { return };
        if card.reading || !card.lands {
            return;
        }
        let entries: Vec<Entry> = DeliveryChoice::OFFERED.iter().zip(PICKS).map(|(one, key)| Entry::Choice { id: key, label: one.label().into(), on: card.landing == Some(*one) }).collect();
        let at = drawn(window, cx, Role::ComboBox, "Delivery").map_or_else(|| window.mouse_position(), super::head::below_left);
        let back = Some(self.composer.input.read(cx).focus_handle(cx));
        self.open_button_menu(
            "Delivery",
            entries,
            at,
            back,
            move |this, picked, _, cx| {
                let Some(choice) = PICKS.iter().position(|one| *one == picked).map(|at| DeliveryChoice::OFFERED[at]) else { return };
                if let Some(id) = this.shown.clone() {
                    this.starts.delivered.insert(id, choice);
                    cx.notify();
                }
            },
            window,
            cx,
        );
    }

    /// Send on a held Job: the box is the first message, and Start begins the agent.
    pub(super) fn begin_start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(card) = self.held_card(&id, cx) else { return };
        if card.starting {
            return;
        }
        // While the proposer reads, the start is queued with no landing: Fleet starts it with the reading's once it ends.
        // The workflow shown is the one the Job runs under, so a default he did not change is recorded now.
        if !card.reading && !card.none && card.kind.is_some() && self.record(&id, cx).is_some_and(|job| job.workflow.is_none()) {
            self.choose_workflow(card.kind.clone(), window, cx);
        }
        let text = self.kept(cx).text;
        let message = Some(text.trim().to_string()).filter(|text| !text.is_empty());
        let delivery = card.lands.then_some(card.landing).flatten();
        self.starts.starting.insert(id.clone());
        cx.notify();
        if self.fleet.is_none() {
            // Mock data: Fleet's yes, as a Fleet that took it would answer. The Job is working and the box is empty.
            self.starts.starting.remove(&id);
            self.started_clears(&id, &text, window, cx);
            self.list.update(cx, |list, cx| list.began(&id, cx));
            return;
        }
        let body = ipc::encode(&BeginSession { session_id: SessionId::carried(id.to_string()), delivery, message });
        let owner = id.clone();
        self.post(
            "/sessions/begin".into(),
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                this.starts.starting.remove(&owner);
                match answer {
                    Ok(_) => this.started_clears(&owner, &text, window, cx),
                    // The box was never emptied, so what he wrote is still there to send again.
                    Err(why) => cx.defer(move |cx| crate::toasts::tell(why, cx)),
                }
                cx.notify();
            }),
        );
    }

    /// Fleet took the start: the brief it was sent leaves the Job's box and its kept draft. Words he typed after
    /// pressing Start stay.
    fn started_clears(&mut self, id: &SharedString, sent: &str, window: &mut Window, cx: &mut Context<Self>) {
        if still_sent(&self.draft_of(id).text, sent) {
            self.keep_draft(id, Kept::default(), cx);
        }
        if self.shown.as_ref() == Some(id) && still_sent(&self.kept(cx).text, sent) {
            self.set_box(&Kept::default(), window, cx);
        }
    }
}

/// Whether the words in a box are still the brief a start sent, so the start may take them away.
fn still_sent(now: &str, sent: &str) -> bool {
    now == sent
}

#[cfg(test)]
mod tests {
    use super::still_sent;

    #[test]
    fn a_started_brief_leaves_the_box_and_words_typed_after_start_stay() {
        let brief = "Handoff from Stage 1: carry the alerts across Macs.";
        // Fleet took it and the box still holds what was sent: the box and the draft empty.
        assert!(still_sent(brief, brief));
        // He typed on after pressing Start: those words are his next message and stay.
        assert!(!still_sent("Handoff from Stage 1: carry the alerts across Macs. Also the dock.", brief));
        assert!(!still_sent("Something else entirely", brief));
        // An empty box has nothing to take away from a brief that was sent.
        assert!(!still_sent("", brief));
    }
}
