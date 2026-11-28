//! The one compact card a Job holds in its thread for the builds to try.
//!
//! **A Job's builds to try are one thing, not one card each** (the owner's take B, 16 Nov 2026). While any build is open or put
//! off the thread draws one compact card pinned at its end, after the items, as the card of what waits for the Job's Mac is: the
//! green band, and a row for each build still to answer, with a state dot, what it is about on one line and a word on the right.
//! The thread items of those builds draw nothing; a build he answered, or the Job withdrew or replaced, stays one line at its own
//! time, and once no build is left to answer every one reads where it happened. A press on a row, or Enter, opens that build whole
//! under the rows, in place, and the thread follows its tail to the end, never up into the history. The review overlay will host
//! the same detail (`ready_detail`).

use std::collections::HashMap;

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, pulsating_between, Animation, AnimationExt, AnyElement, Context, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::{Instant, ReadyState, ThreadItem, ThreadReadyToTry};

use super::acts::Answer;
use super::band_card::{frame, Act};
use super::ready_card::{clock, pending, Parts, Preview, RefreshKeys, Standing, NOT_IN_PREVIEW};
use super::rows::{glyph, tip, Size};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

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


/// The open build after `from` in the list's order, else the first open one that is not `from`.
pub(super) fn next_open(all: &[Build], from: &str) -> Option<String> {
    let open = |build: &&Build| build.standing == Standing::Open && build.card.id != from;
    let after = all.iter().position(|build| build.card.id == from).and_then(|at| all[at + 1..].iter().find(open));
    after.or_else(|| all.iter().find(open)).map(|build| build.card.id.clone())
}

/// When a build put off comes back, as he just chose it or as Fleet holds it.
pub(super) fn until(card: &ThreadReadyToTry, answers: &HashMap<String, Answer>) -> Option<Instant> {
    match answers.get(&card.id) {
        Some(Answer::Later(until)) => Some(Instant::carried(until.clone())),
        _ => card.later_until.clone(),
    }
}

/// What Fleet says of each build and where its item stands, so the thread measures again where one changed.
pub(super) fn signature(items: &[ThreadItem]) -> Vec<(usize, String, ReadyState)> {
    items.iter().enumerate().filter_map(|(at, item)| if let ThreadItem::ReadyToTry(card) = item { Some((at, card.id.clone(), card.state)) } else { None }).collect()
}

/// The first thread item whose build differs between two signatures.
pub(super) fn first_change(was: &[(usize, String, ReadyState)], now: &[(usize, String, ReadyState)]) -> Option<usize> {
    let apart = |one: &&(usize, String, ReadyState), other: &[(usize, String, ReadyState)]| !other.contains(one);
    was.iter().filter(|one| apart(one, now)).chain(now.iter().filter(|one| apart(one, was))).map(|(at, ..)| *at).min()
}

/// Whether a build stands open or put off, as Fleet's states alone say: the compact card is drawn then.
pub(super) fn asks(items: &[ThreadItem]) -> bool {
    items.iter().any(|item| matches!(item, ThreadItem::ReadyToTry(card) if matches!(card.state, ReadyState::Open | ReadyState::Later)))
}

/// The first picture of each build still listed on the card, read from the Job's own tree as a `Read` call's are.
pub(super) fn shots(items: &[ThreadItem]) -> Vec<ipc::SentFile> {
    items
        .iter()
        .filter_map(|item| match item {
            ThreadItem::ReadyToTry(card) if matches!(card.state, ReadyState::Open | ReadyState::Later) => card.pictures.first().and_then(|path| super::pictures::read_picture(path)),
            _ => None,
        })
        .collect()
}

/// The word at the right of a build's row.
fn word_of(build: &Build, preview: &Preview, answers: &HashMap<String, Answer>, now: DateTime<Local>) -> String {
    match build.standing {
        Standing::Open if pending(build.standing, preview) => preview.word().unwrap_or(NOT_IN_PREVIEW).to_string(),
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

/// Whether `extra` is the card the builds to try stand in: their own compact card, or the first card while a build is the first ask.
pub(super) fn holds_builds(extra: &super::Extra) -> bool {
    match extra {
        super::Extra::Ready => true,
        super::Extra::First(key) => key.starts_with(super::first_card::FIRST_BUILD),
        _ => false,
    }
}

impl Page {
    /// The element the compact card is drawn at, after the thread's items, while a build is open or put off.
    pub(super) fn ready_at(&self) -> Option<usize> {
        self.extras.iter().position(holds_builds).map(|at| self.items + at)
    }
}

impl JobView {
    /// Whether the thread pins the Ready to try card now: the one test `extras_of` pushes `Extra::Ready` on. Where it does, the
    /// Needs you band leaves Fleet's Ready to try row out, since the card is already on the thread (17 Nov).
    pub(super) fn ready_pinned_for(&self, id: &str, cx: &gpui::App) -> bool {
        self.thread(id, cx).is_some_and(|held| asks(&held.items))
    }

    /// The Job's one compact card: the green band and a row for each build still to answer. The build he opened unfolds whole under
    /// its own row. Not `framed`, it is the first card's body: the Refresh keys and the rows, with no frame or band of its own.
    pub(super) fn ready_compact(&self, all: &[Build], ix: usize, page: &Page, look: &Look, now: DateTime<Local>, framed: bool, cx: &Context<Self>) -> AnyElement {
        let listed: Vec<&Build> = all.iter().filter(|build| build.standing.asks()).collect();
        let standing = if listed.iter().any(|build| build.standing == Standing::Open) { Standing::Open } else { Standing::Later };
        let newest = listed.iter().max_by_key(|build| build.at).map(|build| build.card);
        let job = self.shown.clone().unwrap_or_default();
        let ids: Vec<String> = listed.iter().map(|build| build.card.id.clone()).collect();
        let previews: Vec<Preview> = listed.iter().map(|build| self.preview_of(build.card, cx)).collect();
        let open = || listed.iter().zip(&previews).filter(|(build, _)| build.standing == Standing::Open).map(|(_, preview)| preview);
        let waits = standing == Standing::Open && !open().any(|preview| *preview == Preview::In);
        // While no open build is in the preview, the head says the nearest: one Refresh would bring in, else one still checked, else one off the list.
        let near = |preview: &&Preview| match preview {
            Preview::NotIn => 0,
            Preview::Checking => 1,
            _ => 2,
        };
        let nearest = open().min_by_key(near);
        let word: SharedString = match nearest.and_then(Preview::word) {
            Some(word) if waits => word.into(),
            _ => standing.word().into(),
        };
        let pending_tip = nearest.filter(|_| waits).map(Preview::tip);
        // The Refresh keys stand while any listed build is outside the preview, and take a press only once one is ready for it.
        let refresh = previews.iter().filter(|preview| **preview != Preview::In).min_by_key(near).map(|preview| RefreshKeys { ready: preview.refreshes(), why: preview.tip() });
        let head = match framed {
            true => newest.map(|card| self.ready_head("thread", standing, word, &card.at, None, pending_tip, refresh, now, look, cx)),
            false => refresh.map(|keys| self.refresh_strip("thread", keys, look, cx)),
        };
        let picked = page.cards.ready_pick.as_deref();
        let wide = f32::from(self.pane.unwrap_or(gpui::px(720.)) - look.length(token::SPACE_4) * 2.);
        let rows: Vec<AnyElement> = listed
            .iter()
            .zip(previews)
            .enumerate()
            .map(|(at, (build, preview))| {
                let card = build.card;
                let open = picked == Some(card.id.as_str());
                let spot = page.keys.spot(&format!("ready-row:{}", card.id), cx);
                let waits = pending(build.standing, &preview);
                let ink = look.color(if waits { token::STATUS_AWAITING_REVIEW } else { build.standing.ink() });
                let word = word_of(build, &preview, &page.cards.answers, now);
                let said = format!("{}, {word}", card.about);
                let (pressed, keyed, opened, ids, neighbours) = (card.id.clone(), card.id.clone(), job.clone(), ids.clone(), job.clone());
                let dot_tip: SharedString = if waits { preview.tip() } else { build.standing.tip().into() };
                let dot = labelled(
                    div().id(SharedString::from(format!("ready-dot:{}", card.id))).flex_none().tooltip(tip(dot_tip.clone())).child(glyph(look, dot_of(build.standing), Size::Mark, ink)),
                    Role::Image,
                    dot_tip,
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
                    .when(open, |row| row.bg(look.color(token::SURFACE_ROW_SELECTED)))
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
                    .child(glyph(look, if open { "chevron-down" } else { "chevron-right" }, Size::Small, look.color(token::FG_MUTED)))
                    .on_click(cx.listener(move |this, _, _, cx| this.ready_open(&opened, &pressed, cx)))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.modifiers.modified() {
                            return;
                        }
                        match event.keystroke.key.as_str() {
                            "enter" | "space" => {
                                cx.stop_propagation();
                                this.ready_open(&neighbours, &keyed, cx);
                            }
                            "escape" if this.ready_opened(&neighbours) => {
                                cx.stop_propagation();
                                this.ready_close(&neighbours, cx);
                            }
                            digit @ ("1" | "2" | "3") => this.ready_digit(&neighbours, digit, window, cx),
                            key @ ("up" | "down") => {
                                cx.stop_propagation();
                                let to = if key == "down" { (at + 1).min(ids.len() - 1) } else { at.saturating_sub(1) };
                                let spot = this.pages.get(&neighbours).and_then(|page| page.keys.spot_of(&format!("ready-row:{}", ids[to])));
                                if let Some(spot) = spot {
                                    if to != at {
                                        crate::ui_sounds::moved();
                                    }
                                    window.focus(&spot.focus, cx);
                                }
                            }
                            _ => {}
                        }
                    }));
                // The build he opened reads right under its own row, as the row unfolding, not at the card's foot.
                let detail = open.then(|| self.ready_detail_inside(card, ix, wide, page, now, look, cx));
                div().flex().flex_col().child(labelled(row, Role::Button, said)).children(detail).into_any_element()
            })
            .collect();
        if !framed {
            return labelled(div().id("ready-compact").flex().flex_col().children(head).child(div().flex().flex_col().py(look.length(token::SPACE_1)).children(rows)), Role::Group, "Ready to try")
                .into_any_element();
        }
        let ink = look.color(standing.ink());
        // While the preview rebuilds, from a Refresh preview pressed anywhere, the whole card breathes in the running hue.
        let refreshing = self.preview_refreshing(cx).is_some();
        let wash = look.color(token::STATUS_RUNNING);
        let group = labelled(
            frame(look, false)
                .id("ready-compact")
                .relative()
                .border_color(if refreshing { wash.opacity(0.6) } else { ink.opacity(0.45) })
                .children(head)
                .child(div().flex().flex_col().py(look.length(token::SPACE_1)).children(rows))
                .when(refreshing, |card| {
                    card.child(div().absolute().inset_0().with_animation(
                        "ready-refreshing",
                        Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(pulsating_between(0.03, 0.1)),
                        move |breath, at| breath.bg(wash.opacity(at)),
                    ))
                }),
            Role::Group,
            "Ready to try",
        );
        div().id("ready-column").flex().flex_col().child(group).into_any_element()
    }

    /// A press on a build's row: it unfolds under that row and the thread follows its tail to the card at its end; a press
    /// on the open one shuts it again.
    pub(super) fn ready_open(&mut self, job: &str, id: &str, cx: &mut Context<Self>) {
        let Some(page) = self.pages.get_mut(job) else { return };
        let Some(at) = page.ready_at() else { return };
        let shut = page.cards.ready_pick.as_deref() == Some(id);
        page.cards.ready_pick = (!shut).then(|| id.to_string());
        page.remeasure(at);
        if !shut {
            page.list.set_follow_mode(gpui::FollowMode::Tail);
        }
        cx.notify();
    }

    /// Whether a build of the Job stands open under the compact card's rows.
    fn ready_opened(&self, job: &str) -> bool {
        self.pages.get(job).is_some_and(|page| page.cards.ready_pick.is_some())
    }

    /// Esc on a row shuts the build opened under the rows.
    fn ready_close(&mut self, job: &str, cx: &mut Context<Self>) {
        let Some(page) = self.pages.get_mut(job) else { return };
        let Some(at) = page.ready_at() else { return };
        page.cards.ready_pick = None;
        page.remeasure(at);
        cx.notify();
    }

    /// 1, 2 and 3 on a row answer the build opened under the rows as its buttons do, unless a box is open for his words.
    fn ready_digit(&mut self, job: &str, digit: &str, window: &mut Window, cx: &mut Context<Self>) {
        let act = match digit {
            "1" => Act::Approve,
            "2" => Act::Ask,
            _ => Act::Open,
        };
        let found = {
            let (Some(page), Some(held)) = (self.pages.get(job), self.thread(job, cx)) else { return };
            let Some(pick) = page.cards.ready_pick.as_deref() else { return };
            let all = builds(&held.items, &page.cards.answers);
            let noting = page.cards.modes.contains_key(pick);
            all.iter().find(|build| build.card.id == pick).filter(|build| build.standing.asks() && build.card.in_build && !noting).and_then(|build| page.ready_at().map(|at| (build.card.clone(), at)))
        };
        let Some((card, at)) = found else { return };
        cx.stop_propagation();
        let run = self.ready_run(at, &card, Parts::of(&card));
        run(self, act, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::super::dock::Thing;
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
    fn the_compact_card_is_drawn_while_a_build_is_open_or_put_off() {
        let held = mixed();
        assert!(asks(&held));
        assert!(asks(&[held[0].clone(), held[2].clone(), held[4].clone()]), "a build put off keeps the card");
        assert!(!asks(&[held[0].clone(), held[4].clone()]), "settled builds list nothing");
        assert!(!asks(&[]));
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
        assert!(asks(&held), "Fleet's states alone still say a build is open");
    }

    #[test]
    fn a_change_of_a_build_names_the_first_item_to_measure_again() {
        let held = mixed();
        let was = signature(&held);
        assert_eq!(first_change(&was, &was), None);
        let mut later = held.clone();
        if let ThreadItem::ReadyToTry(card) = &mut later[3] {
            card.state = ReadyState::Answered;
        }
        assert_eq!(first_change(&was, &signature(&later)), Some(3));
        assert_eq!(first_change(&was, &signature(&held[..3])), Some(3), "the builds the thread no longer holds");
    }

    // ---- drawn, on the mock Fleet's `ready-to-try` scenario ---------------------------------------

    fn on_screen<'a>(cx: &'a mut gpui::TestAppContext, job: &str, wide: f32) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        on_screen_high(cx, job, wide, 6000.)
    }

    /// The Job open in a window `high` tall: short enough, the thread scrolls.
    fn on_screen_high<'a>(cx: &'a mut gpui::TestAppContext, job: &str, wide: f32, high: f32) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        on_screen_with(cx, job, wide, high, false)
    }

    /// The Job open with every build in the running preview, so each one takes an answer.
    fn on_screen_all_in_preview<'a>(cx: &'a mut gpui::TestAppContext, job: &str, wide: f32) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        on_screen_with(cx, job, wide, 6000., true)
    }

    fn on_screen_with<'a>(cx: &'a mut gpui::TestAppContext, job: &str, wide: f32, high: f32, all_in_preview: bool) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        on_screen_waiting(cx, job, wide, high, all_in_preview, Vec::new(), false)
    }

    /// The same, with these items also waiting on him beside the scenario's Ready to try row, and, where `settled`, every build answered so the thread pins no card.
    fn on_screen_waiting<'a>(cx: &'a mut gpui::TestAppContext, job: &str, wide: f32, high: f32, all_in_preview: bool, more: Vec<ipc::WaitingItem>, settled: bool) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let mut jobs = crate::mock::jobs("ready-to-try").expect("the fixture decodes");
        for one in jobs.iter_mut().filter(|one| one.id.as_str() == job) {
            one.waiting_for.extend(more.clone());
        }
        let mut threads = crate::mock::threads("ready-to-try").expect("the threads decode");
        for item in threads.values_mut().flat_map(|thread| thread.items.iter_mut()) {
            if let ThreadItem::ReadyToTry(card) = item {
                card.in_build |= all_in_preview;
                if settled {
                    card.state = ipc::ReadyState::Answered;
                }
            }
        }
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(wide), gpui::px(high)));
        let job = job.to_string();
        list.update(cx, |list, cx| list.open(job.into(), cx));
        redraw(cx);
        (view, cx)
    }

    /// The Job open over an idle Fleet whose build report a test sets.
    fn on_screen_over_fleet<'a>(cx: &'a mut gpui::TestAppContext, job: &str) -> (gpui::Entity<crate::fleet::FleetConnection>, &'a mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("ready-to-try").expect("the fixture decodes");
        let threads = crate::mock::threads("ready-to-try").expect("the threads decode");
        let fleet = cx.new(crate::fleet::FleetConnection::idle);
        // Over a connection the view draws what Fleet holds, not the mock's threads, so Fleet holds them.
        fleet.update(cx, |fleet, _| fleet.hold_threads(threads.values().cloned()));
        let list = cx.new(|cx| crate::jobs::JobsList::mock_over(&jobs, fleet.clone(), cx));
        // Under the label root, as the app draws it, so the book holds only the last frame and a test can ask that a button is gone.
        let (_root, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| super::super::parts_fold::Rooted::of(cx.new(|cx| JobView::new(list, Some(threads), window, cx)))
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(6000.)));
        let job = job.to_string();
        list.update(cx, |list, cx| list.open(job.into(), cx));
        redraw(cx);
        (fleet, cx)
    }

    fn build_report(json: &str) -> ipc::FleetBuildReport {
        ipc::decode::<ipc::FleetBuildReport>("build", json.as_bytes()).expect("a report")
    }

    #[gpui::test]
    fn refresh_preview_on_the_card_reads_refreshing_while_a_rebuild_pressed_anywhere_runs_and_comes_back_after(cx: &mut gpui::TestAppContext) {
        let (fleet, cx) = on_screen_over_fleet(cx, "tool-edit");
        fleet.update(cx, |fleet, cx| fleet.build_said(build_report(r#"{"on":"preview"}"#), cx));
        redraw(cx);
        assert!(refresh_in(cx, "Ready to try"), "with no rebuild under way the card offers Refresh preview");
        // Pressed in another Job's card, the title bar or Needs you: all Fleet's report says is that the preview rebuilds.
        fleet.update(cx, |fleet, cx| fleet.build_said(build_report(r#"{"on":"preview","restarting":"preview","stage":"building_fleet"}"#), cx));
        redraw(cx);
        assert!(!refresh_in(cx, "Ready to try"), "no second press while it rebuilds");
        assert!(drawn(cx, Role::Status, "Refreshing"), "the card's key reads Refreshing");
        fleet.update(cx, |fleet, cx| fleet.build_said(build_report(r#"{"on":"preview"}"#), cx));
        redraw(cx);
        assert!(refresh_in(cx, "Ready to try") && !drawn(cx, Role::Status, "Refreshing"), "Fleet back on the new build, the key is Refresh preview again");
    }

    const CLAIMED: &str = r#"{"on":"preview","refresh":{"mac":"air","name":"Air","stage":"building_fleet","at":"2026-11-19T09:00:00Z"}}"#;

    #[gpui::test]
    fn a_refresh_another_mac_started_reads_refreshing_on_the_card_and_takes_no_press_until_it_is_cleared(cx: &mut gpui::TestAppContext) {
        let (fleet, cx) = on_screen_over_fleet(cx, "tool-edit");
        cx.update(|_, cx| crate::pairing::hold(crate::pairing::mock_macs(true), cx));
        fleet.update(cx, |fleet, cx| fleet.build_said(build_report(r#"{"on":"preview"}"#), cx));
        redraw(cx);
        assert!(key_in(cx, "Refresh both Macs", "Ready to try"), "with no claim the card offers Refresh both Macs");
        // The claim alone: this Mac has no restart under way, so only the claim says the other Mac pressed.
        fleet.update(cx, |fleet, cx| fleet.build_said(build_report(CLAIMED), cx));
        redraw(cx);
        assert!(!key_in(cx, "Refresh both Macs", "Ready to try"), "no second press while another Mac's Refresh stands");
        assert!(drawn(cx, Role::Status, "Refreshing"), "the card's key reads Refreshing");
        assert!(drawn(cx, Role::Status, "Started on Air"), "and names the Mac that started it");
        fleet.update(cx, |fleet, cx| fleet.build_said(build_report(r#"{"on":"preview"}"#), cx));
        redraw(cx);
        assert!(key_in(cx, "Refresh both Macs", "Ready to try") && !drawn(cx, Role::Status, "Refreshing"), "claim cleared, the key is Refresh both Macs again");
    }

    #[gpui::test]
    fn a_rebuild_onto_main_leaves_refresh_preview_as_it_is(cx: &mut gpui::TestAppContext) {
        let (fleet, cx) = on_screen_over_fleet(cx, "tool-edit");
        fleet.update(cx, |fleet, cx| fleet.build_said(build_report(r#"{"on":"preview","restarting":"main"}"#), cx));
        redraw(cx);
        assert!(!drawn(cx, Role::Status, "Refreshing"), "only a preview rebuild reads Refreshing");
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
        let mut groups = drawn_like(cx, Role::Group, "Ready to try");
        groups.dedup();
        assert_eq!(groups, ["Ready to try"], "one card for the Job");
        for (about, word) in [(FULL, "Ready"), (SHOT, "Not in your preview yet"), (BARE, "Ready")] {
            assert!(drawn(cx, Role::Button, &format!("{about}, {word}")), "{about} is a row of the card");
        }
        assert!(drawn_like(cx, Role::Article, "Ready to try:").is_empty(), "no build has a card of its own until one is opened");
        for gone in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(!drawn(cx, Role::Button, gone), "the compact card has no {gone}");
        }
        assert!(drawn(cx, Role::ListItem, "Ready to try: Chip takes the stale colour, Looks good"), "the answered build stays one line");
    }

    #[gpui::test]
    fn a_ready_row_alone_draws_no_band_while_the_card_stands_on_the_thread(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.read_with(cx, |view, cx| {
            assert!(view.ready_pinned_for("tool-edit", cx), "the thread pins the card");
            let held = view.needs_of("tool-edit", cx).is_some_and(|needs| needs.sections.iter().flat_map(|section| &section.rows).any(|row| row.source == ipc::WaitingSource::ReadyToTry));
            assert!(held, "Fleet still gives the row, so the Jobs list still reads the Job as needing him");
            assert!(view.groups("tool-edit", cx).is_empty(), "the row is left out of the band's sections");
        });
        assert!(drawn(cx, Role::Group, "Ready to try"), "the card is on the thread");
        assert!(!drawn(cx, Role::Button, "Needs you"), "nothing else waits, so no band");
        assert!(drawn_like(cx, Role::Region, "To review").is_empty());
    }

    #[gpui::test]
    fn a_question_keeps_the_band_which_leaves_the_ready_row_out(cx: &mut gpui::TestAppContext) {
        let wire = r#"{"id":"pick","text":"Which colour","since":"2026-11-16T10:00:00Z","source":"agent","act":{"kind":"answer","target":"pick"},"options":[{"label":"Blue"},{"label":"Green"}]}"#;
        let question = ipc::decode::<ipc::WaitingItem>("item", wire.as_bytes()).expect("an item decodes");
        let (view, cx) = on_screen_waiting(cx, "tool-edit", 1400., 6000., false, vec![question], false);
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        redraw(cx);
        view.read_with(cx, |view, cx| {
            let (things, review) = view.things("tool-edit", cx);
            assert!(review.is_empty(), "no page waits for review");
            // The sheet holds the question, so it is listed as one line that opens the sheet on it.
            assert!(matches!(things.as_slice(), [Thing::Sheet(item)] if item.id == "pick"), "only the question is listed");
            assert!(!view.band_reviews("tool-edit", cx), "the card alone is no reason for the review sheet");
        });
        assert!(drawn(cx, Role::Button, "Needs you"), "the question keeps the band");
        assert!(drawn(cx, Role::Region, "Questions"));
        assert!(drawn_like(cx, Role::Region, "To review").is_empty(), "no To review section with the card on the thread");
        assert!(drawn(cx, Role::Group, "Ready to try"), "the card is still on the thread");
    }

    /// Whether the thread shows its end: the last element's foot is in view.
    fn at_its_end(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> bool {
        view.read_with(cx, |view, _| {
            let page = view.pages.get("tool-edit").expect("a page");
            let last = page.list.bounds_for_item(page.count - 1);
            page.list.is_following_tail() || last.is_some_and(|bounds| bounds.bottom() <= page.list.viewport_bounds().bottom())
        })
    }

    fn to_the_top(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) {
        view.update(cx, |view, _| {
            let page = view.pages.get("tool-edit").expect("a page");
            page.list.set_follow_mode(gpui::FollowMode::Normal);
            page.list.scroll_to(gpui::ListOffset { item_ix: 0, offset_in_item: gpui::px(0.) });
        });
        redraw(cx);
        assert!(!at_its_end(view, cx), "the thread is read from its head");
    }

    #[gpui::test]
    fn the_open_card_is_an_extra_after_every_item_and_the_builds_items_draw_nothing(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen_high(cx, "tool-edit", 1400., 700.);
        view.read_with(cx, |view, cx| {
            let page = view.pages.get("tool-edit").expect("a page");
            let held = view.thread("tool-edit", cx).expect("a thread");
            let first_build = held.items.iter().position(|item| matches!(item, ThreadItem::ReadyToTry(_))).expect("builds");
            assert!(page.items - first_build >= 10, "the Job went on talking after its builds: {} items after the first", page.items - first_build);
            let ready = page.extras.iter().position(super::holds_builds).expect("the card is an extra");
            assert!(
                page.extras[ready + 1..].iter().all(|extra| matches!(extra, crate::job_view::Extra::Working | crate::job_view::Extra::Subagents | crate::job_view::Extra::Pending(_) | crate::job_view::Extra::Refreshing)),
                "only what the Job does now stands after it: {:?}",
                page.extras
            );
            assert_eq!(page.ready_at(), Some(page.items + ready));
        });
    }

    #[gpui::test]
    fn a_press_on_a_row_in_a_long_thread_leaves_the_list_at_its_end_and_never_goes_up(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen_high(cx, "tool-edit", 1400., 700.);
        to_the_top(&view, cx);
        let shot = id_of(&view, SHOT, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &shot, cx));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(shot));
        assert!(at_its_end(&view, cx), "the thread follows its tail to the card");
        assert!(drawn(cx, Role::Group, "Ready to try"), "the card is in sight");
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {SHOT}")), "and the build opened under its rows");
    }

    /// The review panel's chosen build, and how many builds it lists.
    fn panel_of(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> Option<(String, usize)> {
        view.read_with(cx, |view, _| {
            let review = view.review().filter(|review| review.open)?;
            let chosen = view.review_chosen()?;
            assert_eq!(chosen.kind, super::super::review::Kind::Build, "the panel holds the Job's builds");
            Some((chosen.id, review.groups.iter().map(|group| group.items.len()).sum()))
        })
    }

    #[gpui::test]
    fn the_jobs_ready_to_try_item_opens_the_review_panel_with_every_open_build_and_the_newest_chosen(cx: &mut gpui::TestAppContext) {
        // While a build is open the card is pinned and Needs you draws no row for it, so the item comes in from For you or Now.
        let (view, cx) = on_screen_high(cx, "tool-edit", 1400., 700.);
        view.update_in(cx, |view, window, cx| view.open_artifact("tool-edit", "try:tool-edit", window, cx));
        redraw(cx);
        assert_eq!(panel_of(&view, cx), Some((id_of(&view, BARE, cx), 3)), "the three open builds are listed, the newest chosen");
        assert!(drawn(cx, Role::Region, BARE), "its detail stands in the panel");
        for answer in ["Looks good", "Ask for changes"] {
            assert!(drawn(cx, Role::Button, answer), "with its answer {answer}");
        }
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert_eq!(panel_of(&view, cx), None, "Esc closes the panel");
    }

    #[gpui::test]
    fn a_try_id_and_a_builds_own_id_open_the_review_panel_on_that_build(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.open_artifact("tool-edit", "try:tool-edit", window, cx));
        redraw(cx);
        assert_eq!(panel_of(&view, cx), Some((id_of(&view, BARE, cx), 3)), "the Job's row chooses its newest open build");
        view.update_in(cx, |view, window, cx| view.close_review(window, cx));
        let shot = id_of(&view, SHOT, cx);
        assert!(view.update_in(cx, |view, window, cx| view.open_named(&shot, window, cx)));
        redraw(cx);
        assert_eq!(panel_of(&view, cx), Some((shot, 3)), "a build's own id chooses it");
        assert!(drawn(cx, Role::Region, SHOT), "its detail stands in the panel");
        view.update_in(cx, |view, window, cx| view.close_review(window, cx));
        assert!(!view.update_in(cx, |view, window, cx| view.open_named("try:native/gone@0000", window, cx)), "a build the thread no longer holds opens nothing");
        assert_eq!(panel_of(&view, cx), None);
    }

    #[gpui::test]
    fn an_answer_in_the_panel_is_sent_with_the_build_and_the_next_open_build_comes_up(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.open_artifact("tool-edit", "try:tool-edit", window, cx));
        redraw(cx);
        press(cx, Role::Button, "Looks good");
        view.read_with(cx, |view, _| {
            let sent = view.mock_ready.last().expect("the answer is sent");
            assert_eq!((sent.answer.clone(), sent.branch.as_str()), (ReadyAnswer::LooksGood, "native/reviewer-brief"));
        });
        let bare = id_of(&view, BARE, cx);
        let chosen = panel_of(&view, cx).map(|(id, _)| id).expect("the panel stays up");
        assert_ne!(chosen, bare, "the next open build is chosen");
        assert!([id_of(&view, FULL, cx), id_of(&view, SHOT, cx)].contains(&chosen));
    }

    #[gpui::test]
    fn a_build_that_is_the_first_ask_stands_in_the_first_card_and_nowhere_else(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let first = view.read_with(cx, |view, cx| view.first_ask("tool-edit", cx)).expect("a build is open");
        assert_eq!(first.kind, super::super::review::Kind::Build, "the Job's first ask is a build");
        let extras = view.read_with(cx, |view, _| view.pages.get("tool-edit").expect("a page").extras.clone());
        assert!(!extras.contains(&crate::job_view::Extra::Ready), "no card of the builds' own stands beside the first: {extras:?}");
        assert!(view.read_with(cx, |view, _| view.pages.get("tool-edit").expect("a page").ready_at().is_some()), "the first card is where the builds are");
        let mut groups = drawn_like(cx, Role::Group, "Ready to try");
        groups.dedup();
        assert_eq!(groups, ["Ready to try"], "the builds stand once, in the first card");
        assert!(drawn(cx, Role::ListItem, "Now: Try the build"), "under the step bar");
    }

    #[gpui::test]
    fn a_build_put_off_keeps_the_card_at_the_end(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen_high(cx, "tool-done", 1400., 700.);
        view.read_with(cx, |view, _| assert!(view.pages.get("tool-done").expect("a page").ready_at().is_some()));
    }

    #[gpui::test]
    fn once_no_build_is_open_the_card_leaves_the_end_and_every_build_reads_where_it_happened(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen_all_in_preview(cx, "tool-edit", 1400.);
        let full = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &full, cx));
        redraw(cx);
        for _ in 0..3 {
            view.update_in(cx, |view, window, cx| view.ready_digit("tool-edit", "1", window, cx));
            redraw(cx);
        }
        view.read_with(cx, |view, _| assert_eq!(view.pages.get("tool-edit").expect("a page").ready_at(), None));
        for about in ["Chip takes the stale colour", FULL, SHOT, BARE] {
            assert!(!drawn_like(cx, Role::ListItem, &format!("Ready to try: {about}")).is_empty(), "{about} is a line where it happened");
        }
    }

    #[gpui::test]
    fn a_press_on_a_row_opens_that_build_in_place_and_a_second_press_shuts_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let shot = id_of(&view, SHOT, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &shot, cx));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(shot.clone()));
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {SHOT}")), "the build is drawn whole under the rows");
        let full = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &full, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {FULL}")), "another row takes its place");
        assert!(drawn(cx, Role::Button, "Looks good"));
        assert_eq!(pick_of(&view, cx), Some(full.clone()));
        view.update(cx, |view, cx| view.ready_open("tool-edit", &full, cx));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), None, "a press on the open row shuts it");
    }

    #[gpui::test]
    fn an_open_build_reads_inside_the_one_card_under_one_head(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let shot = id_of(&view, SHOT, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &shot, cx));
        redraw(cx);
        let (heads, inside) = cx.update(|window, cx| {
            let all = cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).to_vec();
            let within = |outer: &crate::labels::Drawn, inner: &crate::labels::Drawn| {
                let (o, i) = (outer.bounds, inner.bounds);
                i.origin.x >= o.origin.x && i.origin.y >= o.origin.y && i.origin.x + i.size.width <= o.origin.x + o.size.width && i.origin.y + i.size.height <= o.origin.y + o.size.height
            };
            let build = all.iter().find(|one| one.role == Role::Article && one.name.as_ref() == format!("Ready to try: {SHOT}")).expect("the open build is drawn");
            let heads = all.iter().filter(|one| one.role == Role::Status && within(build, one)).count();
            let inside = all.iter().any(|one| one.role == Role::Group && one.name.as_ref() == "Ready to try" && within(one, build));
            (heads, inside)
        });
        assert_eq!(heads, 0, "the card's one head says its state; the open build draws no head of its own");
        assert!(inside, "the open build is drawn inside the Ready to try card");
    }

    #[gpui::test]
    fn the_detail_is_the_build_whole_with_the_four_answers(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 2400.);
        let full = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &full, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Button, "You approved: Sketch, Merge line flow"));
        assert!(drawn(cx, Role::Button, "sketch-cards.png"));
        assert!(drawn(cx, Role::Heading, "What changed and where to look"));
        assert!(drawn(cx, Role::List, "Steps"));
        for name in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(drawn(cx, Role::Button, name), "{name} is on the detail");
        }
    }

    #[gpui::test]
    fn an_answer_moves_the_open_build_to_the_next_open_one(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let full = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &full, cx));
        redraw(cx);
        let one = |view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext| {
            view.update_in(cx, |view, window, cx| view.ready_digit("tool-edit", "1", window, cx));
            redraw(cx);
        };
        one(&view, cx);
        view.read_with(cx, |view, _| assert!(matches!(view.mock_ready.last().map(|sent| &sent.answer), Some(ReadyAnswer::LooksGood))));
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, SHOT, cx)), "the next open build is opened");
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {SHOT}")));
        let sent = view.read_with(cx, |view, _| view.mock_ready.len());
        one(&view, cx);
        view.read_with(cx, |view, _| assert_eq!(view.mock_ready.len(), sent, "a build not in the preview takes no answer"));
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, SHOT, cx)));
        let bare = id_of(&view, BARE, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &bare, cx));
        redraw(cx);
        one(&view, cx);
        view.read_with(cx, |view, cx| {
            let held = view.thread("tool-edit", cx).expect("a thread");
            assert!(asks(&held.items), "the build not in the preview is still listed");
            assert!(held.items.iter().any(|item| matches!(item, ThreadItem::ReadyToTry(card) if card.about == SHOT)));
        });
    }

    #[gpui::test]
    fn a_build_answered_withdrawn_or_replaced_still_folds_to_one_line_and_a_put_off_one_is_a_row(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-done", 1400.);
        for line in [
            "Ready to try: Clock sits under the chip, Needs rework",
            "Ready to try: Arrows with words, Withdrawn",
            "Ready to try: Rail layout for the Jobs list, Replaced by a newer build",
        ] {
            assert!(drawn(cx, Role::ListItem, line), "{line} stands as a line");
        }
        assert!(!drawn_like(cx, Role::Button, "Toast pile drawn as a stack, Later until ").is_empty(), "a build put off says when it comes back, as a row");
        assert!(drawn_like(cx, Role::ListItem, "Ready to try: Toast pile").is_empty(), "and is no line of its own");
        for gone in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(!drawn(cx, Role::Button, gone), "the thread has no {gone} button");
        }
    }

    #[gpui::test]
    fn a_refused_answer_takes_it_back_and_says_fleets_sentence_on_the_build(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let full = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &full, cx));
        redraw(cx);
        let (held, at) = view.read_with(cx, |view, cx| {
            let held = view.thread("tool-edit", cx).expect("a thread");
            let card = held.items.iter().find_map(|item| match item {
                ThreadItem::ReadyToTry(card) if card.about == FULL => Some(card.clone()),
                _ => None,
            });
            (card.expect("the card"), view.pages.get("tool-edit").and_then(|page| page.ready_at()).expect("a place"))
        });
        let past = Instant::carried((Local::now() - chrono::Duration::hours(1)).to_rfc3339());
        view.update_in(cx, |view, window, cx| view.answer_ready(at, &held, ReadyAnswer::Remind { until: past }, window, cx));
        redraw(cx);
        let said = view.read_with(cx, |view, _| view.pages.get(view.shown.as_ref().expect("shown")).and_then(|page| page.cards.said.get(&held.id).map(|why| why.to_string())));
        assert_eq!(said.as_deref(), Some("a time to remind him at that has not passed"));
        assert!(drawn(cx, Role::Status, "Refused"), "the sentence stands where he answered");
        assert!(drawn(cx, Role::Button, "Looks good"), "the build stays open");
    }

    #[gpui::test]
    fn with_no_card_pinned_needs_you_holds_one_item_for_the_job_with_a_plain_dismiss_and_no_answer(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen_waiting(cx, "tool-edit", 1400., 6000., false, Vec::new(), true);
        view.read_with(cx, |view, cx| assert!(!view.ready_pinned_for("tool-edit", cx), "every build is answered, so the thread pins no card"));
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Button, "Needs you"), "the row is the Job's only way to the builds");
        assert!(!drawn_like(cx, Role::Button, "Open Ready to try").is_empty(), "one item stands for the Job, however many builds");
        assert!(!drawn_like(cx, Role::Button, "Remove Ready to try").is_empty(), "a plain dismiss puts it away");
        assert!(drawn_like(cx, Role::Button, "Answer Ready to try").is_empty(), "no disabled Answer stands on it");
    }

    /// Whether a Refresh preview button is drawn inside the labelled element whose name starts with `inside`.
    fn refresh_in(cx: &mut gpui::VisualTestContext, inside: &str) -> bool {
        key_in(cx, "Refresh preview", inside)
    }

    fn key_in(cx: &mut gpui::VisualTestContext, key: &str, inside: &str) -> bool {
        cx.update(|window, cx| {
            let id = window.window_handle().window_id();
            cx.global::<crate::labels::LabelBook>().drawn(id).iter().any(|one| one.role == Role::Button && one.name == key && one.path.iter().any(|name| name.starts_with(inside)))
        })
    }

    #[gpui::test]
    fn refresh_preview_stands_on_the_card_head_while_a_listed_build_is_not_in_the_preview(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-edit", 1400.);
        assert!(drawn(cx, Role::Button, "Refresh preview"), "the card head offers it");
        assert!(refresh_in(cx, "Ready to try"), "inside the card");
    }

    #[gpui::test]
    fn refresh_preview_stands_under_the_card_head_and_not_in_the_built_frame_of_an_opened_build(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let outside = id_of(&view, SHOT, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &outside, cx));
        redraw(cx);
        assert!(refresh_in(cx, "Ready to try"), "the card's head offers it");
        assert!(!refresh_in(cx, &format!("Ready to try: {SHOT}")), "the built picture's frame only says where the build stands");
        assert!(drawn(cx, Role::Status, "Restarts Fleet on latest main."), "what Refresh builds stands under it");
    }

    #[gpui::test]
    fn a_build_not_in_the_preview_says_so_and_takes_no_answer(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let outside = id_of(&view, SHOT, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &outside, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Button, &format!("{SHOT}, Not in your preview yet")), "its row, right above it, says it");
        assert_eq!(super::super::ready_card::built_label(&Preview::NotIn).0, "Not in your preview yet", "and so does the built picture's frame");
        for name in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(!drawn(cx, Role::Button, name), "{name} waits until the preview holds the build");
        }
        assert!(refresh_in(cx, "Ready to try"), "Refresh preview stays");
    }

    #[gpui::test]
    fn the_same_build_in_the_preview_says_built_and_takes_the_answers(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let inside = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &inside, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Button, &format!("{FULL}, Ready")), "its row is the plain one");
        assert_eq!(super::super::ready_card::built_label(&Preview::In).0, "Built \u{b7} in the preview");
        for name in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(drawn(cx, Role::Button, name), "{name} is on a build the preview holds");
        }
    }

    #[gpui::test]
    fn the_compact_row_of_a_build_not_in_the_preview_says_so(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-edit", 1400.);
        assert!(drawn(cx, Role::Button, &format!("{SHOT}, Not in your preview yet")), "its row says it");
        assert!(drawn(cx, Role::Button, &format!("{FULL}, Ready")), "a build the preview holds is Ready");
        assert!(drawn(cx, Role::Image, "The running preview does not hold this build yet"), "its dot says it");
    }

    /// Fleet's offers holding the build titled `about`, at its branch and tip, in `state`.
    fn offers_of(about: &str, state: &str, why: Option<&str>) -> ipc::PreviewOffers {
        let threads = crate::mock::threads("ready-to-try").expect("the threads decode");
        let card = threads
            .values()
            .flat_map(|thread| &thread.items)
            .find_map(|item| match item {
                ThreadItem::ReadyToTry(card) if card.about == about => Some(card.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("the thread holds a build titled {about}"));
        let why = why.map_or(String::new(), |why| format!(r#","why":"{why}""#));
        let wire = format!(
            r#"{{"offers":[{{"branch":"{}","tip":"{}","session_id":"tool-edit","title":"Tool edit","about":"{about}","offered_at":"2026-11-15T14:10:00Z","state":"{state}","in_build":false{why}}}]}}"#,
            card.branch, card.tip
        );
        ipc::decode::<ipc::PreviewOffers>("offers", wire.as_bytes()).expect("offers")
    }

    fn offers_held(fleet: &gpui::Entity<crate::fleet::FleetConnection>, offers: ipc::PreviewOffers, cx: &mut gpui::VisualTestContext) {
        fleet.update(cx, |fleet, cx| {
            fleet.hold_offers(offers);
            cx.notify();
        });
        redraw(cx);
    }

    const CHECKING_TIP: &str = "Fleet is merging this build onto main and compiling it. Refresh preview can take it once that passes";

    #[gpui::test]
    fn a_build_fleet_is_still_checking_says_so_and_its_refresh_is_off(cx: &mut gpui::TestAppContext) {
        let (fleet, cx) = on_screen_over_fleet(cx, "tool-edit");
        offers_held(&fleet, offers_of(SHOT, "checking", None), cx);
        assert!(drawn(cx, Role::Button, &format!("{SHOT}, Checking that the build will work")), "its row says Fleet is still checking it");
        assert!(!drawn(cx, Role::Button, &format!("{SHOT}, Not in your preview yet")), "not a word that sends him to Refresh");
        assert!(refresh_in(cx, "Ready to try"), "Refresh stands where he looks for it");
        assert!(drawn(cx, Role::Status, CHECKING_TIP), "off, and under it why");
        assert!(drawn_like(cx, Role::Status, "Restarts Fleet").is_empty(), "no promise of a rebuild that would leave the branch out");
    }

    #[gpui::test]
    fn a_build_whose_check_failed_is_off_the_list_with_fleets_why_and_its_refresh_is_off(cx: &mut gpui::TestAppContext) {
        let (fleet, cx) = on_screen_over_fleet(cx, "tool-edit");
        let why = "Fleet's check failed it: conflicts with main in src/queue.rs";
        offers_held(&fleet, offers_of(SHOT, "failed", Some("conflicts with main in src/queue.rs")), cx);
        assert!(drawn(cx, Role::Button, &format!("{SHOT}, Off the preview list")), "its row says the check failed it");
        assert!(drawn(cx, Role::Image, why), "its dot says why");
        assert!(drawn(cx, Role::Status, why), "and so does the line under the Refresh it cannot use");
        assert!(drawn_like(cx, Role::Status, "Restarts Fleet").is_empty(), "Refresh cannot bring it in");
    }

    #[gpui::test]
    fn a_build_fleet_checked_ready_offers_refresh_with_what_it_builds(cx: &mut gpui::TestAppContext) {
        let (fleet, cx) = on_screen_over_fleet(cx, "tool-edit");
        offers_held(&fleet, offers_of(SHOT, "ready", None), cx);
        assert!(drawn(cx, Role::Button, &format!("{SHOT}, Not in your preview yet")), "ready and not built reads as before");
        assert!(refresh_in(cx, "Ready to try"), "Refresh brings it in");
        assert!(drawn(cx, Role::Status, "Restarts Fleet on latest main with the offered branches."), "the line under Refresh says what it builds");
        assert!(!drawn(cx, Role::Status, CHECKING_TIP), "nothing holds it off");
    }

    #[test]
    fn only_a_build_fleet_checked_ready_takes_a_refresh() {
        assert!(Preview::NotIn.refreshes());
        for off in [Preview::In, Preview::Checking, Preview::Off(None)] {
            assert!(!off.refreshes(), "{off:?}");
        }
    }

    #[gpui::test]
    fn a_build_in_the_preview_has_no_refresh_beside_its_built_chip(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let inside = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &inside, cx));
        redraw(cx);
        assert!(!refresh_in(cx, &format!("Ready to try: {FULL}")), "nothing to refresh");
    }
}
