//! A feature's or a sub project's board, in the right column where a Job opens: its heading pressed, or Enter on it. Its title,
//! what it is, the sub projects that hold a feature, and What holds what: a row for each Job in it saying what that Job waits on
//! now, as its record says it, with Open. A sub project's board has Ungroup. ↑ and ↓ move between them, Enter presses, Esc leaves.

use super::leads::Lead;
use super::*;
use crate::parts::button::{button, Kind};

/// A Job on the board: what its row draws.
struct Holding {
    id: SharedString,
    title: SharedString,
    glyph: &'static str,
    hue: crate::look::Token,
    state: SharedString,
    holds: SharedString,
}

/// What the board draws, read from the list.
struct Read {
    title: SharedString,
    line: SharedString,
    chips: Vec<SharedString>,
    rows: Vec<Holding>,
    ungroup: bool,
}

pub(crate) struct LeadBoard {
    list: gpui::WeakEntity<JobsList>,
    lead: Lead,
    focus: FocusHandle,
    /// The stop the keys are on: each row's Open by its place, then Ungroup.
    cursor: usize,
    refused: Option<SharedString>,
    sending: bool,
    _watch: Subscription,
}

impl LeadBoard {
    pub(super) fn new(list: &Entity<JobsList>, lead: Lead, cx: &mut Context<Self>) -> Self {
        let watch = cx.observe(list, |_, _, cx| cx.notify());
        LeadBoard { list: list.downgrade(), lead, focus: cx.focus_handle(), cursor: 0, refused: None, sending: false, _watch: watch }
    }

    pub(super) fn focus(&self) -> &FocusHandle {
        &self.focus
    }

    #[cfg(test)]
    pub(super) fn lead(&self) -> &Lead {
        &self.lead
    }

    fn read(&self, cx: &App) -> Option<Read> {
        let list = self.list.upgrade()?;
        let list = list.read(cx);
        let live: Vec<&SessionRecord> = list.jobs.iter().filter(|job| job.state == SessionState::Live).collect();
        let holding = |job: &SessionRecord| {
            let (glyph, state) = mark_of(job).map(|mark| glyph_of(mark.state, None)).unwrap_or(("circle-dot", SharedString::default()));
            Holding {
                id: job.id.as_str().to_string().into(),
                title: title_of(job),
                glyph,
                hue: mark_of(job).map_or(token::FG_MUTED, |mark| hue_of(mark.state)),
                state,
                holds: leads::holds(job).into(),
            }
        };
        match &self.lead {
            Lead::Feature { project, slug } => {
                let (feature, area) = list.leads.feature(project, slug)?;
                let chips = list.leads.subs.iter().filter(|sub| sub.project == *project && sub.features.iter().any(|one| one == slug)).map(|sub| sub.name.clone().into()).collect();
                let rows = feature.jobs_now.iter().filter_map(|one| live.iter().find(|job| job.id == one.session_id)).map(|job| holding(job)).collect();
                Some(Read { title: feature.title.clone().into(), line: format!("Feature \u{b7} {area}").into(), chips, rows, ungroup: false })
            }
            Lead::SubProject { id } => {
                let sub = list.leads.subs.iter().find(|sub| sub.id == *id)?;
                let titles = list.leads.feature_titles(sub);
                let line = if titles.is_empty() { "Sub project".to_string() } else { format!("Sub project \u{b7} {}", titles.join(", ")) };
                let rows = list.leads.members(sub, &live).into_iter().map(|(job, _)| holding(job)).collect();
                Some(Read { title: sub.name.clone().into(), line: line.into(), chips: Vec::new(), rows, ungroup: true })
            }
            Lead::Area | Lead::NoFeature => None,
        }
    }

    /// Press the stop at `at`: a row's Open opens its Job, the last stop on a sub project's board is Ungroup.
    fn press(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(read) = self.read(cx) else { return };
        let Some(list) = self.list.upgrade() else { return };
        match read.rows.get(at) {
            Some(row) => {
                let id = row.id.clone();
                list.update(cx, |list, cx| list.open(id, cx));
            }
            None if read.ungroup => self.ungroup(window, cx),
            None => {}
        }
    }

    /// Ungroup: Fleet drops the sub project, its heading leaves and the keys go back to the list. A refusal is said on the board.
    fn ungroup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Lead::SubProject { id }, false) = (self.lead.clone(), self.sending) else { return };
        let Some(list) = self.list.upgrade() else { return };
        self.sending = true;
        self.refused = None;
        let sent = list.update(cx, |list, cx| list.ungroup_sub_project(&id, cx));
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.sending = false;
                match answer {
                    Ok(()) => list.update(cx, |list, cx| list.close_board(window, cx)),
                    Err(why) => this.refused = Some(why.into()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn on_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let stops = self.read(cx).map_or(0, |read| read.rows.len() + usize::from(read.ungroup));
        match event.keystroke.key.as_str() {
            "down" | "j" => self.cursor = (self.cursor + 1).min(stops.saturating_sub(1)),
            "up" | "k" => self.cursor = self.cursor.saturating_sub(1),
            "enter" => self.press(self.cursor, window, cx),
            "escape" => {
                if let Some(list) = self.list.upgrade() {
                    list.update(cx, |list, cx| list.close_board(window, cx));
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Render for LeadBoard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let Some(read) = self.read(cx) else { return div().size_full().into_any_element() };
        let keyed = self.focus.is_focused(window);
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let cursor = self.cursor.min((read.rows.len() + usize::from(read.ungroup)).saturating_sub(1));
        let rows = read.rows.len();
        let chips = (!read.chips.is_empty()).then(|| {
            div().flex().flex_wrap().gap(look.length(token::SPACE_2)).children(read.chips.iter().enumerate().map(|(at, name)| {
                labelled(crate::parts::chip::chip(look, ("board-sub", at), look.color(token::ACCENT), Some("folder-tree"), name.clone()), Role::Label, format!("In {name}"))
            }))
        });
        let ungroup = read.ungroup.then(|| {
            labelled(
                button(look, "board-ungroup", Kind::Secondary, None, "Ungroup", self.sending)
                    .when(keyed && cursor == rows, |button| button.border_color(ring))
                    .when(!self.sending, |button| button.on_click(cx.listener(|this, _, window, cx| this.ungroup(window, cx)))),
                Role::Button,
                "Ungroup",
            )
        });
        let refusal = self.refused.clone().map(|why| labelled(div().id("board-refused").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::ERROR)).child(why.clone()), Role::Alert, why));
        let holds = read.rows.into_iter().enumerate().map(|(at, row)| {
            let open = labelled(
                button(look, ("board-open", at), Kind::Secondary, None, "Open", false)
                    .when(keyed && cursor == at, |button| button.border_color(ring))
                    .on_click(cx.listener(move |this, _, window, cx| this.press(at, window, cx))),
                Role::Button,
                format!("Open {}", row.title),
            );
            let state = row.state.clone();
            labelled(
                div()
                    .id(("board-row", at))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .when(at > 0, |line| line.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                    .child(labelled(
                        div()
                            .id(("board-state", at))
                            .flex_none()
                            .child(svg().path(icon_path(row.glyph)).size(look.length(token::SPACE_4)).text_color(look.color(row.hue)))
                            .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(state.clone()).build(window, cx)),
                        Role::Image,
                        row.state.clone(),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_1))
                            .child(div().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child(row.title.clone()))
                            .child(div().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(row.holds.clone())),
                    )
                    .child(open),
                Role::Row,
                format!("{}, {}", row.title, row.holds),
            )
        });
        let card = crate::parts::card::card(look, "what-holds-what", None)
            .child(crate::parts::card::head(look, "what-holds-what-head", None, "What holds what"))
            .child(crate::parts::card::body(look, "what-holds-what-body").gap(look.length(token::SPACE_1)).children(holds));
        labelled(
            div()
                .id("lead-board")
                .key_context("board")
                .track_focus(&self.focus)
                .on_key_down(cx.listener(Self::on_key))
                .size_full()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .p(look.length(token::SPACE_6))
                .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                .text_color(look.color(token::FG_DEFAULT))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .child(labelled(
                            div().id("board-title").text_size(look.length(token::TEXT_XL)).line_height(look.length(token::LEADING_XL)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(read.title.clone()),
                            Role::Heading,
                            read.title.clone(),
                        ))
                        .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(read.line.clone()))
                        .children(chips),
                )
                .children(ungroup.map(|button| div().flex().child(button)))
                .children(refusal)
                .child(card),
            Role::Region,
            read.title,
        )
        .into_any_element()
    }
}
