//! The Workflow tab: a Job's lanes as Fleet works them out (`SessionRecord::lanes`), drawn by the shared
//! canvas (`crate::lanes_canvas`) as Classic's `LaneCanvas` draws them in the Job's Workflow tab
//! (`shared-panel/lanes.tsx`). What a press means on a Job is here: a ticket head's menu, a started Job's
//! lanes opened over these, and a rule's acts.
//!
//! A Job with no lanes draws nothing in the tab.

use gpui::{div, prelude::*, AnyElement, App, Context, Div, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent, Role, SharedString, Stateful, Window};
use ipc::{JobLanesView, LaneAutonomy, LaneView};

use super::rows::{glyph, mono, Size};
use super::JobView;
use crate::labels::labelled;
use crate::lanes_canvas::{stamp, LanesHost, LanesState, Pick};
use crate::look::{token, Look};

/// The Workflow tab's state on the Job view.
pub(super) struct Lanes {
    /// The canvas: its frame's keys and each Job's view of it.
    pub canvas: LanesState,
    /// Add to this Job, Changed on this Job, and the Jobs opened over the lanes.
    pub copy: super::lanes_copy::LanesCopy,
}

impl Lanes {
    pub fn new(cx: &mut gpui::App) -> Self {
        Self { canvas: LanesState::new(cx), copy: super::lanes_copy::LanesCopy::new(cx) }
    }
}

impl LanesHost for JobView {
    fn lanes_state(&self) -> &LanesState {
        &self.lanes.canvas
    }

    fn lanes_of<'a>(&'a self, id: &SharedString, cx: &'a App) -> Option<&'a JobLanesView> {
        self.record(id, cx)?.lanes.as_ref()
    }

    fn lanes_picked(&mut self, id: &SharedString, pick: Option<Pick>, window: &mut Window, cx: &mut Context<Self>) {
        self.rule_picked(id, pick, window, cx);
    }

    fn lanes_reveal(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.reveal_field("canvas", window, cx);
    }

    fn head_opens(&self, lanes: &JobLanesView, lane: &LaneView, cx: &App) -> bool {
        self.lane_opens(lanes, &lane.id, cx)
    }

    /// A ticket's head opens a menu with Teach; the head of a Job this one started, where that Job has
    /// lanes, opens them on a double press or Enter.
    fn head_words(&self, id: &SharedString, lane: &LaneView, opens: bool, words: Stateful<Div>, look: &Look, cx: &Context<Self>) -> AnyElement {
        if lane.ticket {
            let handle = self.head_focus(id, &lane.id, cx);
            let (title, note, held) = (lane.title.clone(), lane.note.clone(), lane.id.clone());
            let (title_k, note_k, held_k) = (title.clone(), note.clone(), held.clone());
            let named = format!("Ticket {}", lane.title);
            let named_k = named.clone();
            let pressed = handle.clone();
            labelled(
                words
                    .track_focus(&handle)
                    .cursor_pointer()
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.stop_propagation();
                        window.focus(&pressed, cx);
                    })
                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.open_ticket_head_menu(title.clone(), note.clone(), held.clone(), event.position, false, window, cx);
                    }))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        let keys = &event.keystroke;
                        if keys.key == "menu" || keys.key == "contextmenu" || (keys.key == "f10" && keys.modifiers.shift) {
                            cx.stop_propagation();
                            let at = super::head::drawn(window, cx, Role::Button, &named_k).map_or_else(|| window.mouse_position(), super::head::below_left);
                            this.open_ticket_head_menu(title_k.clone(), note_k.clone(), held_k.clone(), at, true, window, cx);
                        }
                    })),
                Role::Button,
                named,
            )
            .into_any_element()
        } else if opens {
            let handle = self.head_focus(id, &lane.id, cx);
            let (parent, held, parent_k, held_k) = (id.clone(), lane.id.clone(), id.clone(), lane.id.clone());
            let pressed = handle.clone();
            labelled(
                words
                    .track_focus(&handle)
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.stop_propagation();
                        window.focus(&pressed, cx);
                    })
                    .cursor_pointer()
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                        if event.click_count() == 2 {
                            this.lanes_into(&parent, &held, window, cx);
                        }
                    }))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "enter" && !event.keystroke.modifiers.modified() {
                            cx.stop_propagation();
                            this.lanes_into(&parent_k, &held_k, window, cx);
                        }
                    })),
                Role::Button,
                format!("Open Workflow: {}", lane.title),
            )
            .into_any_element()
        } else {
            words.into_any_element()
        }
    }
}

impl JobView {
    /// A Job's canvas: the rail, then the lanes with their heads over the left (`LaneCanvas`).
    pub(super) fn lanes_canvas(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let lanes = self.record(id, cx)?.lanes.as_ref()?;
        Some(crate::lanes_canvas::lanes_canvas(self, id, lanes, look, cx))
    }

    /// ⌥W: the Workflow tab forward, with the keys on its canvas (`focusKind`).
    pub(super) fn lanes_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab_closed("workflow") {
            self.add_tab("workflow", window, cx);
        } else {
            self.zone = super::side::Zone::Workflow;
            cx.notify();
        }
        let frame = self.lanes.canvas.frame.clone();
        window.defer(cx, move |window, cx| window.focus(&frame, cx));
    }
}

/// How an act stands, as Fleet keeps it: the glyph the rule's chip uses for the same, and the word (`STANDING`).
fn standing(of: &str) -> (&'static str, String) {
    match of {
        "planned" => ("clock", "Planned".into()),
        "asked" => ("message-circle-question-mark", "Asked".into()),
        "stuck" => ("message-circle-question-mark", "Stuck".into()),
        "done" => ("check", "Done".into()),
        "skipped" => ("ban", "Skipped".into()),
        "told" => ("check", "Told the agent".into()),
        "not_carried" => ("clock", "Not carried".into()),
        other => ("clock", other.to_string()),
    }
}

impl JobView {
    /// A rule's name in the Job's copy, else its id.
    pub(super) fn rule_label(&self, id: &SharedString, rule: &str, cx: &Context<Self>) -> String {
        self.record(id, cx).and_then(|job| job.lanes.as_ref()).and_then(|lanes| lanes.job.copy.rules.iter().find(|one| one.id == rule)).map_or_else(|| rule.to_string(), |one| one.label.clone())
    }

    /// Whether a rule kept acts on some instance, so that picking it opens them.
    fn rule_has_acts(&self, id: &SharedString, rule: &str, cx: &Context<Self>) -> bool {
        self.record(id, cx).and_then(|job| job.lanes.as_ref()).is_some_and(|lanes| lanes.job.fired.iter().any(|one| one.rule == rule && !one.acts.is_empty()))
    }

    /// A rule picked opens what it did, where Fleet kept acts for it (`pick` in `OwnLanes`).
    fn rule_picked(&mut self, id: &SharedString, pick: Option<Pick>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Pick::Rule(rule)) = pick {
            if self.rule_has_acts(id, &rule, cx) {
                self.open_rule_panel(id.clone(), rule, window, cx);
            }
        }
    }

    /// What a rule did, a framed list for each instance it fired on (`ActsPanel`).
    pub(super) fn rule_body(&self, id: &SharedString, rule: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(lanes) = self.record(id, cx).and_then(|job| job.lanes.as_ref()) else { return div().into_any_element() };
        let job = &lanes.job;
        let label = self.rule_label(id, rule, cx);
        let sections: Vec<AnyElement> = job
            .fired
            .iter()
            .filter(|one| one.rule == rule && !one.acts.is_empty())
            .map(|one| {
                let held = job.instances.iter().find(|each| each.id == one.instance).map_or_else(|| label.clone(), |each| each.label.clone());
                let rows: Vec<AnyElement> = one
                    .acts
                    .iter()
                    .enumerate()
                    .map(|(at, act)| {
                        let (icon, said) = standing(&act.standing);
                        let quiet = matches!(act.standing.as_str(), "skipped" | "not_carried");
                        let small = |div: gpui::Div| div.text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS));
                        let mono_fact = |text: String| small(div().flex_none().flex().items_center().h(look.length(token::H_CONTROL_SM)).font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).whitespace_nowrap()).child(SharedString::from(text));
                        div()
                            .flex()
                            .items_start()
                            .gap(look.length(token::SPACE_2))
                            .px(look.length(token::SPACE_3))
                            .py(look.length(token::SPACE_1))
                            .when(at > 0, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                            .child(
                                small(div().flex_none().w(look.length(token::SPACE_12) * 3.5).h(look.length(token::H_CONTROL_SM)).flex().items_center().gap(look.length(token::SPACE_2)).text_color(look.color(token::FG_MUTED)).whitespace_nowrap())
                                    .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
                                    .child(SharedString::from(said)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(small(div().min_h(look.length(token::H_CONTROL_SM)).flex().items_center().text_color(look.color(if quiet { token::FG_SUBTLE } else { token::FG_MUTED }))).child(SharedString::from(act.text.clone())))
                                    .children(act.said.as_ref().map(|why| small(div().mb(look.length(token::SPACE_1)).text_color(look.color(token::FG_SUBTLE))).child(SharedString::from(why.clone())))),
                            )
                            .child(mono_fact(if act.course == LaneAutonomy::Asks { "Asks".into() } else { "Acts".into() }))
                            .child(mono_fact(stamp(&act.at)).id(SharedString::from(format!("act-at:{}:{at}", one.instance))).tooltip(super::rows::tip("Time")))
                            .into_any_element()
                    })
                    .collect();
                labelled(
                    div()
                        .id(SharedString::from(format!("rule-firing:{}", one.instance)))
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .rounded(look.length(token::RADIUS_LG))
                        .bg(look.color(token::BG_SUNKEN))
                        .child(
                            div()
                                .px(look.length(token::SPACE_3))
                                .py(look.length(token::SPACE_1))
                                .border_b(look.length(token::BORDER_WIDTH))
                                .border_color(look.color(token::BORDER_DEFAULT))
                                .bg(look.color(token::BG_RAISED))
                                .font_family(mono(look))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .text_color(look.color(token::FG_DEFAULT))
                                .child(SharedString::from(held.clone())),
                        )
                        .child(labelled(div().id(SharedString::from(format!("rule-acts:{}", one.instance))).flex().flex_col().children(rows), Role::List, held.clone())),
                    Role::Group,
                    held,
                )
                .into_any_element()
            })
            .collect();
        div().flex().flex_col().gap(look.length(token::SPACE_3)).min_w_0().p(look.length(token::SPACE_6)).children(sections).into_any_element()
    }
}
