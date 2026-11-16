//! The Ready to try tab and the one compact card a Job holds in its thread.
//!
//! **A Job's builds to try are one thing, not one card each** (the owner's take B, 16 Nov 2026). The thread draws one compact card
//! at the newest open build's place: the green band, and a row for each build still to answer, with a state dot, what it is about
//! on one line and a word on the right. The older thread items of those builds draw nothing; a build he answered, or the Job
//! withdrew or replaced, stays one line at its own time. A press on a row, or Enter, opens this tab with that build picked: the
//! list of every build of the Job on the left, the picked one drawn whole on the right, built the way the review sheet's list and
//! detail are.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::{DateTime, Local};
use gpui::{canvas, div, prelude::*, AnyElement, Context, FontWeight, KeyDownEvent, Pixels, Render, Role, SharedString, WeakEntity, Window};
use ipc::{Instant, ReadyState, ThreadItem, ThreadReadyToTry};

use super::acts::{Answer, SHEET};
use super::band_card::{frame, Act};
use super::ready_card::{clock, Parts, Standing};
use super::rows::{glyph, mono, tip, Size};
use super::view_tabs::ViewTab;
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// The tab's id among a Job's view tabs.
pub(super) const TAB: &str = "ready-to-try";

/// The key of the list's focus handle.
pub(super) const LIST: &str = "ready-list";

/// One build of a Job: where it stands in the thread, the card, and how it stands now.
pub(super) struct Build<'a> {
    pub at: usize,
    pub card: &'a ThreadReadyToTry,
    pub standing: Standing,
}

/// Every build in the thread, in the order the tab lists them: the open ones first, then the ones put off, each oldest first, then
/// the settled ones, newest first.
pub(super) fn builds<'a>(items: &'a [ThreadItem], answers: &HashMap<String, Answer>) -> Vec<Build<'a>> {
    let mut all: Vec<Build> = items
        .iter()
        .enumerate()
        .filter_map(|(at, item)| match item {
            ThreadItem::ReadyToTry(card) => Some(Build { at, card, standing: Standing::of(card, answers.get(&card.id)) }),
            _ => None,
        })
        .collect();
    let rank = |build: &Build| match build.standing {
        Standing::Open => 0,
        Standing::Later => 1,
        _ => 2,
    };
    all.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| if rank(a) == 2 { b.at.cmp(&a.at) } else { a.at.cmp(&b.at) }));
    all
}

/// The thread item the compact card stands at: the newest open build's, else the newest put off.
pub(super) fn place(all: &[Build]) -> Option<usize> {
    let newest = |standing: Standing| all.iter().filter(|build| build.standing == standing).map(|build| build.at).max();
    newest(Standing::Open).or_else(|| newest(Standing::Later))
}

/// The open build after `from` in the list's order, else the first open one that is not `from`.
pub(super) fn next_open(all: &[Build], from: &str) -> Option<String> {
    let open = |build: &&Build| build.standing == Standing::Open && build.card.id != from;
    let after = all.iter().position(|build| build.card.id == from).and_then(|at| all[at + 1..].iter().find(open));
    after.or_else(|| all.iter().find(open)).map(|build| build.card.id.clone())
}

/// The build the tab draws: the one picked if the Job still holds it, else the first open one, else the first.
pub(super) fn picked<'a, 'b>(all: &'b [Build<'a>], held: Option<&str>) -> Option<&'b Build<'a>> {
    held.and_then(|id| all.iter().find(|build| build.card.id == id)).or_else(|| all.iter().find(|build| build.standing == Standing::Open)).or_else(|| all.first())
}

/// When a build put off comes back, as he just chose it or as Fleet holds it.
pub(super) fn until(card: &ThreadReadyToTry, answers: &HashMap<String, Answer>) -> Option<Instant> {
    match answers.get(&card.id) {
        Some(Answer::Later(until)) => Some(Instant::carried(until.clone())),
        _ => card.later_until.clone(),
    }
}

/// What Fleet says of each build, so the thread measures again where one changed.
pub(super) fn signature(items: &[ThreadItem]) -> Vec<(String, ReadyState)> {
    items.iter().filter_map(|item| if let ThreadItem::ReadyToTry(card) = item { Some((card.id.clone(), card.state)) } else { None }).collect()
}

/// The thread item the compact card stands at, as Fleet's states alone say.
pub(super) fn place_in(items: &[ThreadItem]) -> Option<usize> {
    place(&builds(items, &HashMap::new()))
}

/// The word at the right of a build's row.
fn word_of(build: &Build, answers: &HashMap<String, Answer>, now: DateTime<Local>) -> String {
    match build.standing {
        Standing::Open => "Ready".to_string(),
        Standing::Later => until(build.card, answers).map_or_else(|| "Later".to_string(), |at| format!("Later until {}", clock(&at, now))),
        other => other.word().to_string(),
    }
}

/// The glyph of a state dot.
fn dot_of(standing: Standing) -> &'static str {
    match standing {
        Standing::Open => "circle-dot",
        Standing::Later => "clock",
        Standing::Good => "circle-check",
        Standing::Rework => "pencil",
        Standing::Withdrawn => "x",
        Standing::Replaced => "refresh-cw",
    }
}

/// The tab, as the pane draws it: the Job view draws what is in it, and this holds the width its pane has, which says whether
/// the two pictures fit side by side.
pub(super) struct ReadyTab {
    view: WeakEntity<JobView>,
    job: SharedString,
    width: Rc<Cell<Pixels>>,
}

impl ReadyTab {
    fn new(view: WeakEntity<JobView>, job: SharedString) -> Self {
        ReadyTab { view, job, width: Rc::new(Cell::new(gpui::px(0.))) }
    }
}

impl Render for ReadyTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (me, width) = (cx.entity_id(), self.width.clone());
        let held = width.get();
        let job = self.job.clone();
        let drawn = self.view.update(cx, |view, cx| view.render_ready_tab(&job, held, window, cx)).ok().flatten();
        // The pane's width is read as it paints; a change draws this tab again with the new one.
        let measure = canvas(
            |bounds, _, _| bounds.size.width,
            move |_, wide, _, cx| {
                if width.replace(wide) != wide {
                    cx.notify(me);
                }
            },
        )
        .absolute()
        .size_full();
        div().size_full().relative().child(measure).children(drawn)
    }
}

impl JobView {
    /// Open the Ready to try tab of Job `job` beside its thread, with the build `pick` picked, or the first open one. A tab open
    /// already comes forward. Whether the Job holds the build asked for.
    pub(super) fn open_ready_tab(&mut self, job: &str, pick: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let key = SharedString::from(job.to_string());
        let chosen = {
            let (Some(page), Some(held)) = (self.pages.get(&key), self.thread(job, cx)) else { return false };
            let all = builds(&held.items, &page.cards.answers);
            match pick {
                Some(id) => all.iter().find(|build| build.card.id == id).map(|build| build.card.id.clone()),
                None => picked(&all, None).map(|build| build.card.id.clone()),
            }
        };
        let Some(chosen) = chosen else { return false };
        if let Some(page) = self.pages.get_mut(&key) {
            page.cards.ready_pick = Some(chosen);
        }
        if !self.front_view_tab(job, TAB, cx) {
            let weak = cx.weak_entity();
            let view = cx.new(|_| ReadyTab::new(weak, key.clone()));
            let tab = ViewTab { id: TAB.to_string(), title: "Ready to try".into(), glyph: "circle-dot", view: view.into(), menu: Vec::new(), closing: None, closed: None, renamed: None };
            self.open_view_tab(job, tab, true, cx);
        }
        if let Some(list) = self.pages.get(&key).map(|page| page.keys.spot(LIST, cx).focus) {
            window.defer(cx, move |window, cx| window.focus(&list, cx));
        }
        cx.notify();
        true
    }

    /// What a press on a Job's Ready to try item, or on one build, opens: the tab. `try:<session_id>` picks the first open build and
    /// `try:<branch>@<tip>` that build. A Job whose thread holds no such build opens nothing.
    pub(super) fn go_to_ready(&mut self, job: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let pick = (id.strip_prefix("try:") != Some(job)).then_some(id);
        self.open_ready_tab(job, pick, window, cx)
    }

    /// The build the tab has picked and where it stands now, for the shown Job.
    fn picked_ready(&self, job: &str, cx: &Context<Self>) -> Option<(ThreadReadyToTry, Standing)> {
        let page = self.pages.get(job)?;
        let held = self.thread(job, cx)?;
        let all = builds(&held.items, &page.cards.answers);
        picked(&all, page.cards.ready_pick.as_deref()).map(|build| (build.card.clone(), build.standing))
    }

    /// Pick a build of the list.
    fn pick_ready(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if let Some(page) = self.pages.get_mut(&job) {
            page.cards.ready_pick = Some(id.to_string());
            self.body_scroll.set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
            let list = page.keys.spot(LIST, cx).focus;
            window.focus(&list, cx);
        }
        cx.notify();
    }

    /// ↑ and ↓ in the list move the pick by one build, and stop at the ends.
    fn step_ready_pick(&mut self, down: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let next = {
            let (Some(page), Some(held)) = (self.pages.get(&job), self.thread(&job, cx)) else { return };
            let all = builds(&held.items, &page.cards.answers);
            let Some(now) = picked(&all, page.cards.ready_pick.as_deref()).and_then(|one| all.iter().position(|build| build.card.id == one.card.id)) else { return };
            let to = if down { (now + 1).min(all.len() - 1) } else { now.saturating_sub(1) };
            all[to].card.id.clone()
        };
        self.pick_ready(&next, window, cx);
    }

    /// 1, 2 and 3 answer the picked build as the buttons do, unless a box is open for his words.
    fn ready_tab_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() {
            return;
        }
        let act = match event.keystroke.key.as_str() {
            "1" => Act::Approve,
            "2" => Act::Ask,
            "3" => Act::Open,
            _ => return,
        };
        let Some(job) = self.shown.clone() else { return };
        let Some((card, standing)) = self.picked_ready(&job, cx) else { return };
        let noting = self.pages.get(&job).is_some_and(|page| page.cards.modes.contains_key(&card.id));
        if !standing.asks() || noting {
            return;
        }
        cx.stop_propagation();
        let run = self.ready_run(SHEET, &card, Parts::of(&card));
        run(self, act, window, cx);
    }

    /// The Job's one compact card: the green band and a row for each build still to answer.
    pub(super) fn ready_compact(&self, all: &[Build], page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let listed: Vec<&Build> = all.iter().filter(|build| build.standing.asks()).collect();
        let standing = if listed.iter().any(|build| build.standing == Standing::Open) { Standing::Open } else { Standing::Later };
        let newest = listed.iter().max_by_key(|build| build.at).map(|build| build.card);
        let job = self.shown.clone().unwrap_or_default();
        let ids: Vec<String> = listed.iter().map(|build| build.card.id.clone()).collect();
        let head = newest.map(|card| self.ready_head("thread", standing, standing.word().into(), &card.at, None, listed.iter().any(|build| !build.card.in_build), now, look, cx));
        let rows: Vec<AnyElement> = listed
            .iter()
            .enumerate()
            .map(|(at, build)| {
                let card = build.card;
                let spot = page.keys.spot(&format!("ready-row:{}", card.id), cx);
                let ink = look.color(build.standing.ink());
                let word = word_of(build, &page.cards.answers, now);
                let said = format!("{}, {word}", card.about);
                let (pressed, keyed, opened, ids, neighbours) = (card.id.clone(), card.id.clone(), job.clone(), ids.clone(), job.clone());
                let dot = labelled(
                    div().id(SharedString::from(format!("ready-dot:{}", card.id))).flex_none().tooltip(tip(build.standing.tip())).child(glyph(look, dot_of(build.standing), Size::Mark, ink)),
                    Role::Image,
                    build.standing.tip(),
                );
                let row = div()
                    .id(SharedString::from(format!("ready-row:{}", card.id)))
                    .track_focus(&spot.focus)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_2))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER)))
                    .child(dot)
                    .child(
                        div()
                            .id(SharedString::from(format!("ready-about:{}", card.id)))
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .text_color(look.color(token::FG_DEFAULT))
                            .tooltip(tip(card.about.clone()))
                            .child(SharedString::from(card.about.clone())),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .text_color(if build.standing == Standing::Open { ink } else { look.color(token::FG_MUTED) })
                            .child(SharedString::from(word)),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_ready_tab(&opened, Some(&pressed), window, cx);
                    }))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.modifiers.modified() {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "enter" | "space" => {
                                cx.stop_propagation();
                                this.open_ready_tab(&neighbours, Some(&keyed), window, cx);
                            }
                            key @ ("up" | "down") => {
                                cx.stop_propagation();
                                let to = if key == "down" { (at + 1).min(ids.len() - 1) } else { at.saturating_sub(1) };
                                let spot = this.pages.get(&neighbours).and_then(|page| page.keys.spot_of(&format!("ready-row:{}", ids[to])));
                                if let Some(spot) = spot {
                                    window.focus(&spot.focus, cx);
                                }
                            }
                            _ => {}
                        }
                    }));
                labelled(row, Role::Button, said).into_any_element()
            })
            .collect();
        let ink = look.color(standing.ink());
        labelled(
            frame(look, false)
                .id("ready-compact")
                .border_color(ink.opacity(0.45))
                .children(head)
                .child(div().flex().flex_col().py(look.length(token::SPACE_1)).children(rows)),
            Role::Group,
            "Ready to try",
        )
        .into_any_element()
    }

    /// The tab: the list of the Job's builds on the left, the picked one whole on the right.
    pub(super) fn render_ready_tab(&mut self, job: &SharedString, width: Pixels, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.shown.as_ref() != Some(job) {
            return None;
        }
        // The shot the picked build carries is read from the Job's own tree, as a `Read` call's is.
        let (card, _) = self.picked_ready(job, cx)?;
        let shots: Vec<ipc::SentFile> = card.pictures.first().and_then(|path| super::pictures::read_picture(path)).into_iter().collect();
        if !shots.is_empty() {
            self.want_read_pictures(job, &shots, window, cx);
        }
        let look = cx.look();
        let page = self.pages.get(job)?;
        let held = self.thread(job, cx)?;
        let all = builds(&held.items, &page.cards.answers);
        let now = Local::now();
        let at = picked(&all, page.cards.ready_pick.as_deref())?;
        let width = if width > gpui::px(0.) { width } else { gpui::px(720.) };
        let roomy = width >= look.length(token::SPACE_12) * 19.;
        let list_w = if roomy { look.length(token::SPACE_12) * 7. } else { look.length(token::SPACE_12) * 5. + look.length(token::SPACE_6) };

        let mut rows: Vec<AnyElement> = Vec::new();
        let mut settled = false;
        for build in &all {
            if !settled && !build.standing.asks() {
                settled = true;
                rows.push(self.ready_settled_head(look));
            }
            rows.push(self.ready_item(build, at.card.id == build.card.id, page, now, look, cx));
        }
        let list = labelled(
            div()
                .id("ready-list")
                .track_focus(&page.keys.spot(LIST, cx).focus)
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.modifiers.modified() {
                        return;
                    }
                    match event.keystroke.key.as_str() {
                        "down" => this.step_ready_pick(true, window, cx),
                        "up" => this.step_ready_pick(false, window, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .flex_none()
                .w(list_w)
                .h_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_3))
                .overflow_y_scroll()
                .bg(look.color(token::BG_SUNKEN))
                .children(rows),
            Role::ListBox,
            "Builds to try",
        );
        let wide = f32::from(width - list_w - look.length(token::BORDER_WIDTH) - look.length(token::SPACE_6) * 2.);
        let detail = self.ready_detail(at.card, wide, page, now, look, cx);
        let body = div()
            .id("ready-body")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.body_scroll)
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .child(detail);
        Some(
            labelled(
                div()
                    .id("ready-tab")
                    .size_full()
                    .flex()
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.ready_tab_key(event, window, cx)))
                    .child(list)
                    .child(labelled(div().id("ready-detail").flex_1().min_w_0().min_h_0().flex().flex_col().child(body), Role::Region, at.card.about.clone())),
                Role::Region,
                "Ready to try",
            )
            .into_any_element(),
        )
    }

    /// The quiet heading the settled builds stand under.
    fn ready_settled_head(&self, look: &Look) -> AnyElement {
        labelled(
            div()
                .id("ready-settled")
                .px(look.length(token::SPACE_2))
                .pt(look.length(token::SPACE_3))
                .pb(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_SUBTLE))
                .child("Settled"),
            Role::Heading,
            "Settled",
        )
        .into_any_element()
    }

    /// One build of the list: its state dot and what it is about, then where it stands and its time. A press picks it.
    fn ready_item(&self, build: &Build, on: bool, page: &Page, now: DateTime<Local>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let card = build.card;
        let ink = look.color(build.standing.ink());
        let word = word_of(build, &page.cards.answers, now);
        let pick = card.id.clone();
        let settled = !build.standing.asks();
        let dot = labelled(
            div().id(SharedString::from(format!("ready-item-dot:{}", card.id))).flex_none().tooltip(tip(build.standing.tip())).child(glyph(look, dot_of(build.standing), Size::Mark, ink)),
            Role::Image,
            build.standing.tip(),
        );
        let row = div()
            .id(SharedString::from(format!("ready-item:{}", card.id)))
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { look.color(token::BORDER_STRONG) } else { gpui::transparent_black() })
            .when(on, |row| row.bg(look.color(token::BG_RAISED)).shadow(look.shadows(token::SHADOW_CARD)))
            .when(!on, |row| row.hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER))))
            .when(settled, |row| row.opacity(0.72))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(dot)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(card.about.clone())),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .child(div().flex_1().min_w_0().truncate().text_color(if settled { look.color(token::FG_MUTED) } else { ink }).child(SharedString::from(word.clone())))
                    .child(div().flex_none().font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(clock(&card.at, now)))),
            )
            .children(page.cards.said.get(&card.id).cloned().map(|why| {
                div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why)
            }))
            .on_click(cx.listener(move |this, _, window, cx| this.pick_ready(&pick, window, cx)));
        labelled(row, Role::ListBoxOption, format!("{}, {word}", card.about)).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::super::parts_fold::{drawn, drawn_like, redraw};
    use super::*;
    use gpui::Role;
    use ipc::ReadyAnswer;

    fn named(id: &str, about: &str, at: &str, rest: &str) -> ThreadItem {
        let wire = format!(r#"{{"kind":"ready_to_try","id":"try:{id}","at":"{at}","offered_at":"{at}","branch":"{id}","tip":"t","about":"{about}","in_build":true,{rest}}}"#);
        ipc::decode::<ThreadItem>("item", wire.as_bytes()).expect("an item decodes")
    }

    fn ids<'a>(all: &[Build<'a>]) -> Vec<&'a str> {
        all.iter().map(|build| build.card.about.as_str()).collect()
    }

    fn mixed() -> Vec<ThreadItem> {
        vec![
            named("a", "Old answer", "2026-11-15T09:00:00Z", r#""state":"answered","verdict":"good""#),
            named("b", "First open", "2026-11-15T10:00:00Z", r#""state":"open""#),
            named("c", "Put off", "2026-11-15T11:00:00Z", r#""state":"later","later_until":"2026-11-16T09:00:00Z""#),
            named("d", "Second open", "2026-11-15T12:00:00Z", r#""state":"open""#),
            named("e", "Withdrawn one", "2026-11-15T13:00:00Z", r#""state":"withdrawn""#),
        ]
    }

    #[test]
    fn the_list_has_the_open_builds_first_then_the_put_off_ones_then_the_settled_newest_first() {
        let held = mixed();
        let all = builds(&held, &HashMap::new());
        assert_eq!(ids(&all), ["First open", "Second open", "Put off", "Withdrawn one", "Old answer"]);
    }

    #[test]
    fn the_compact_card_stands_at_the_newest_open_build_else_the_newest_put_off() {
        let held = mixed();
        assert_eq!(place_in(&held), Some(3), "the second open build");
        let only_later = vec![held[0].clone(), held[2].clone(), held[4].clone()];
        assert_eq!(place_in(&only_later), Some(1));
        assert_eq!(place_in(&[held[0].clone(), held[4].clone()]), None, "settled builds list nothing");
        assert_eq!(place_in(&[]), None);
    }

    #[test]
    fn the_next_open_build_follows_in_the_lists_order_and_wraps() {
        let held = mixed();
        let all = builds(&held, &HashMap::new());
        assert_eq!(next_open(&all, "try:b").as_deref(), Some("try:d"));
        assert_eq!(next_open(&all, "try:d").as_deref(), Some("try:b"), "past the last open one it wraps");
        assert_eq!(next_open(&all, "try:c").as_deref(), Some("try:b"), "from a build put off it is the first open one");
        let one = vec![held[1].clone()];
        assert_eq!(next_open(&builds(&one, &HashMap::new()), "try:b"), None, "the only open build has no next");
    }

    #[test]
    fn what_he_pressed_moves_a_build_between_the_lists_parts_before_fleet_says_so() {
        let held = mixed();
        let pressed = HashMap::from([("try:b".to_string(), Answer::Approved)]);
        let all = builds(&held, &pressed);
        assert_eq!(ids(&all), ["Second open", "Put off", "Withdrawn one", "First open", "Old answer"]);
        assert_eq!(place_in(&held), Some(3));
        assert_eq!(place(&all), Some(3));
    }

    #[test]
    fn the_pick_is_the_one_held_else_the_first_open_else_the_first() {
        let held = mixed();
        let all = builds(&held, &HashMap::new());
        assert_eq!(picked(&all, Some("try:e")).map(|one| one.card.id.as_str()), Some("try:e"));
        assert_eq!(picked(&all, Some("try:gone")).map(|one| one.card.id.as_str()), Some("try:b"));
        assert_eq!(picked(&all, None).map(|one| one.card.id.as_str()), Some("try:b"));
        let settled = vec![held[0].clone(), held[4].clone()];
        assert_eq!(picked(&builds(&settled, &HashMap::new()), None).map(|one| one.card.id.as_str()), Some("try:e"));
    }

    // ---- drawn, on the mock Fleet's `ready-to-try` scenario ---------------------------------------

    fn on_screen<'a>(cx: &'a mut gpui::TestAppContext, job: &str, wide: f32) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("ready-to-try").expect("the fixture decodes");
        let threads = crate::mock::threads("ready-to-try").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(wide), gpui::px(6000.)));
        let job = job.to_string();
        list.update(cx, |list, cx| list.open(job.into(), cx));
        redraw(cx);
        (view, cx)
    }

    fn press(cx: &mut gpui::VisualTestContext, role: Role, name: &str) {
        let at = cx
            .update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|one| one.bounds.center()))
            .unwrap_or_else(|| panic!("{name} is drawn"));
        cx.simulate_click(at, gpui::Modifiers::none());
        redraw(cx);
    }

    fn id_of(view: &gpui::Entity<JobView>, about: &str, cx: &mut gpui::VisualTestContext) -> String {
        view.read_with(cx, |view, cx| {
            let job = view.shown.clone().expect("a Job is shown");
            let held = view.thread(&job, cx).expect("it holds a thread");
            held.items
                .iter()
                .find_map(|item| match item {
                    ThreadItem::ReadyToTry(card) if card.about == about => Some(card.id.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("the thread holds a build titled {about}"))
        })
    }

    fn pick_of(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> Option<String> {
        view.read_with(cx, |view, _| view.pages.get(view.shown.as_ref().expect("shown")).and_then(|page| page.cards.ready_pick.clone()))
    }

    const FULL: &str = "Sketch boxes drawn as cards";
    const SHOT: &str = "Search finds a Job by its branch";
    const BARE: &str = "The reviewer's brief names its files";

    #[gpui::test]
    fn several_open_builds_are_one_compact_card_and_the_older_items_draw_nothing(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-edit", 1400.);
        assert_eq!(drawn_like(cx, Role::Group, "Ready to try"), ["Ready to try"], "one card for the Job");
        for about in [FULL, SHOT, BARE] {
            assert!(drawn(cx, Role::Button, &format!("{about}, Ready")), "{about} is a row of the card");
        }
        assert!(drawn_like(cx, Role::Article, "Ready to try:").is_empty(), "no build has a card of its own");
        for gone in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(!drawn(cx, Role::Button, gone), "the compact card has no {gone}");
        }
        assert!(drawn(cx, Role::Button, "Ready to try: Chip takes the stale colour, Looks good"), "the answered build stays one line");
    }

    #[gpui::test]
    fn the_compact_card_stands_where_the_newest_open_build_does(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.read_with(cx, |view, cx| {
            let held = view.thread("tool-edit", cx).expect("a thread");
            let at = place_in(&held.items).expect("a place");
            assert!(matches!(&held.items[at], ThreadItem::ReadyToTry(card) if card.about == BARE));
        });
    }

    #[gpui::test]
    fn a_press_on_a_row_opens_the_tab_with_that_build_picked(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        press(cx, Role::Button, &format!("{SHOT}, Ready"));
        view.read_with(cx, |view, _| assert!(view.view_tab_ids("tool-edit").iter().any(|id| id == TAB), "the tab is open"));
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, SHOT, cx)));
        assert!(drawn(cx, Role::Region, "Ready to try"), "the tab draws");
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {SHOT}")), "the picked build whole on the right");
        press(cx, Role::ListBoxOption, &format!("{FULL}, Ready"));
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, FULL, cx)), "a press in the list picks");
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {FULL}")));
    }

    #[gpui::test]
    fn the_detail_is_the_card_whole_with_the_four_answers(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 2400.);
        view.update_in(cx, |view, window, cx| view.open_ready_tab("tool-edit", None, window, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {FULL}")), "the first open build is picked");
        assert!(drawn(cx, Role::Button, "You approved: Sketch, Merge line flow"));
        assert!(drawn(cx, Role::Button, "sketch-cards.png"));
        assert!(drawn(cx, Role::Heading, "What changed and where to look"));
        assert!(drawn(cx, Role::List, "Steps"));
        for name in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(drawn(cx, Role::Button, name), "{name} is on the detail");
        }
        assert!(drawn(cx, Role::Heading, "Settled"), "settled builds stand under their own heading");
        assert!(drawn(cx, Role::ListBoxOption, "Chip takes the stale colour, Looks good"));
    }

    #[gpui::test]
    fn an_answer_moves_the_pick_to_the_next_open_build(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.open_ready_tab("tool-edit", None, window, cx));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, FULL, cx)));
        press(cx, Role::Button, "Looks good");
        view.read_with(cx, |view, _| assert!(matches!(view.mock_ready.last().map(|sent| &sent.answer), Some(ReadyAnswer::LooksGood))));
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, SHOT, cx)), "the next open build is picked");
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {SHOT}")));
        press(cx, Role::Button, "Looks good");
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, BARE, cx)));
        press(cx, Role::Button, "Looks good");
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, BARE, cx)), "with none open the pick stays");
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {BARE}")));
        assert!(!drawn(cx, Role::Button, "Looks good"), "an answered build asks nothing");
        assert!(drawn_like(cx, Role::Group, "Ready to try").is_empty(), "no build is left for the compact card");
    }

    #[gpui::test]
    fn the_arrows_move_the_pick_and_one_answers_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.open_ready_tab("tool-edit", None, window, cx));
        redraw(cx);
        cx.simulate_keystrokes("down");
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, SHOT, cx)), "down picks the next build");
        cx.simulate_keystrokes("up");
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, FULL, cx)));
        cx.simulate_keystrokes("up");
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, FULL, cx)), "up stops at the first");
        cx.simulate_keystrokes("1");
        redraw(cx);
        view.read_with(cx, |view, _| assert!(matches!(view.mock_ready.last().map(|sent| &sent.answer), Some(ReadyAnswer::LooksGood))));
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, SHOT, cx)), "the answer moved the pick");
    }

    #[gpui::test]
    fn the_session_id_form_and_the_branch_form_each_open_the_tab(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.open_artifact("tool-edit", "try:tool-edit", window, cx));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, FULL, cx)), "the Job's item picks its first open build");
        let bare = id_of(&view, BARE, cx);
        assert!(view.update_in(cx, |view, window, cx| view.open_named(&bare, window, cx)));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(bare), "a build's own id picks it");
        assert!(!view.update_in(cx, |view, window, cx| view.open_named("try:native/gone@0000", window, cx)), "a build the thread no longer holds opens nothing");
        assert!(view.read_with(cx, |view, _| view.view_tab_ids("tool-edit").iter().filter(|id| *id == TAB).count() == 1), "one tab for the Job");
    }

    #[gpui::test]
    fn a_build_answered_withdrawn_or_replaced_still_folds_to_one_line_and_a_put_off_one_is_a_row(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-done", 1400.);
        for line in [
            "Ready to try: Clock sits under the chip, Needs rework",
            "Ready to try: Arrows with words, Withdrawn",
            "Ready to try: Rail layout for the Jobs list, Replaced by a newer build",
        ] {
            assert!(drawn(cx, Role::Button, line), "{line} stands as a line");
        }
        assert!(!drawn_like(cx, Role::Button, "Toast pile drawn as a stack, Later until ").is_empty(), "a build put off says when it comes back, as a row");
        assert!(drawn_like(cx, Role::Button, "Ready to try: Toast pile").is_empty(), "and is no line of its own");
        for gone in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(!drawn(cx, Role::Button, gone), "the thread has no {gone} button");
        }
    }

    #[gpui::test]
    fn a_settled_line_opens_the_tab_on_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-done", 1400.);
        press(cx, Role::Button, "Ready to try: Arrows with words, Withdrawn");
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, "Arrows with words", cx)));
        assert!(drawn(cx, Role::Article, "Ready to try: Arrows with words"));
        assert!(!drawn(cx, Role::Button, "Looks good"), "a withdrawn build asks nothing");
    }

    #[gpui::test]
    fn a_refused_answer_takes_it_back_and_says_fleets_sentence_on_the_row(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.open_ready_tab("tool-edit", None, window, cx));
        redraw(cx);
        let held = view.read_with(cx, |view, cx| match view.thread("tool-edit", cx).and_then(|held| held.items.iter().find_map(|item| match item {
            ThreadItem::ReadyToTry(card) if card.about == FULL => Some(card.clone()),
            _ => None,
        })) {
            Some(card) => card,
            None => panic!("the card"),
        });
        let past = Instant::carried((Local::now() - chrono::Duration::hours(1)).to_rfc3339());
        view.update_in(cx, |view, window, cx| view.answer_ready(SHEET, &held, ReadyAnswer::Remind { until: past }, window, cx));
        redraw(cx);
        let said = view.read_with(cx, |view, _| view.pages.get(view.shown.as_ref().expect("shown")).and_then(|page| page.cards.said.get(&held.id).map(|why| why.to_string())));
        assert_eq!(said.as_deref(), Some("a time to remind him at that has not passed"));
        assert!(drawn(cx, Role::Status, "Refused"), "the sentence stands where he answered");
        assert!(drawn(cx, Role::Button, "Looks good"), "the build stays open");
    }

    #[gpui::test]
    fn needs_you_holds_one_item_for_the_job_with_a_plain_dismiss_and_no_answer_and_a_press_opens_the_tab(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        redraw(cx);
        assert!(!drawn_like(cx, Role::Button, "Open Ready to try").is_empty(), "one item stands for the Job, however many builds");
        assert!(!drawn_like(cx, Role::Button, "Remove Ready to try").is_empty(), "a plain dismiss puts it away");
        assert!(drawn_like(cx, Role::Button, "Answer Ready to try").is_empty(), "no disabled Answer stands on it");
        press(cx, Role::Button, "Open Ready to try");
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, FULL, cx)), "the first open build is picked");
        assert!(view.read_with(cx, |view, _| view.view_tab_ids("tool-edit").iter().any(|id| id == TAB)));
    }

    #[gpui::test]
    fn refresh_preview_stands_on_the_card_and_on_a_build_not_in_the_running_preview(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        assert!(drawn(cx, Role::Button, "Refresh preview"), "the card head offers it while a listed build is not in the preview");
        view.update_in(cx, |view, window, cx| view.open_ready_tab("tool-edit", None, window, cx));
        redraw(cx);
        assert!(drawn_like(cx, Role::Button, "Refresh preview").is_empty(), "a build already in the preview has nothing to refresh");
        let outside = id_of(&view, SHOT, cx);
        view.update_in(cx, |view, window, cx| view.open_named(&outside, window, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Button, "Refresh preview"), "a build not in the preview offers Refresh on the tab head");
        press(cx, Role::Button, "Refresh preview");
        assert_eq!(pick_of(&view, cx), Some(outside), "pressing it does not change the pick");
    }
}
