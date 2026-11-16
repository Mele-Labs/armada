//! The one compact card a Job holds in its thread for the builds to try.
//!
//! **A Job's builds to try are one thing, not one card each** (the owner's take B, 16 Nov 2026). The thread draws one compact card
//! at the newest open build's place: the green band, and a row for each build still to answer, with a state dot, what it is about
//! on one line and a word on the right. The older thread items of those builds draw nothing; a build he answered, or the Job
//! withdrew or replaced, stays one line at its own time. A press on a row, or Enter, opens that build whole under the rows, in
//! place, and the thread scrolls to it. The review overlay will host the same detail (`ready_detail`).

use std::collections::HashMap;

use chrono::{DateTime, Local};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::{Instant, ReadyState, ThreadItem, ThreadReadyToTry};

use super::acts::Answer;
use super::band_card::{frame, Act};
use super::ready_card::{clock, Parts, Standing};
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

impl JobView {
    /// The Job's one compact card: the green band and a row for each build still to answer. The build he opened is drawn whole under
    /// the rows, in place.
    pub(super) fn ready_compact(&self, all: &[Build], ix: usize, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
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
        let open = page.cards.ready_pick.as_deref().and_then(|id| listed.iter().find(|build| build.card.id == id));
        let wide = f32::from(self.pane.unwrap_or(gpui::px(720.)) - look.length(token::SPACE_4) * 2.);
        let detail = open.map(|build| self.ready_detail(build.card, ix, wide, page, now, look, cx));
        let group = labelled(
            frame(look, false)
                .id("ready-compact")
                .border_color(ink.opacity(0.45))
                .children(head)
                .child(div().flex().flex_col().py(look.length(token::SPACE_1)).children(rows)),
            Role::Group,
            "Ready to try",
        );
        div().id("ready-column").flex().flex_col().gap(look.length(token::SPACE_3)).child(group).children(detail).into_any_element()
    }

    /// A press on a build's row: it opens in place under the rows and the thread scrolls to the card; a press on the open one shuts
    /// it again.
    pub(super) fn ready_open(&mut self, job: &str, id: &str, cx: &mut Context<Self>) {
        let Some(at) = self.thread(job, cx).and_then(|held| place_in(&held.items)) else { return };
        let Some(page) = self.pages.get_mut(job) else { return };
        let shut = page.cards.ready_pick.as_deref() == Some(id);
        page.cards.ready_pick = (!shut).then(|| id.to_string());
        page.remeasure(at);
        if !shut {
            page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(at), offset_in_item: gpui::px(0.) });
        }
        cx.notify();
    }

    /// Whether a build of the Job stands open under the compact card's rows.
    fn ready_opened(&self, job: &str) -> bool {
        self.pages.get(job).is_some_and(|page| page.cards.ready_pick.is_some())
    }

    /// Esc on a row shuts the build opened under the rows.
    fn ready_close(&mut self, job: &str, cx: &mut Context<Self>) {
        let Some(at) = self.thread(job, cx).and_then(|held| place_in(&held.items)) else { return };
        let Some(page) = self.pages.get_mut(job) else { return };
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
            all.iter().find(|build| build.card.id == pick).filter(|build| build.standing.asks() && !noting).and_then(|build| place(&all).map(|at| (build.card.clone(), at)))
        };
        let Some((card, at)) = found else { return };
        cx.stop_propagation();
        let run = self.ready_run(at, &card, Parts::of(&card));
        run(self, act, window, cx);
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
        let mut groups = drawn_like(cx, Role::Group, "Ready to try");
        groups.dedup();
        assert_eq!(groups, ["Ready to try"], "one card for the Job");
        for about in [FULL, SHOT, BARE] {
            assert!(drawn(cx, Role::Button, &format!("{about}, Ready")), "{about} is a row of the card");
        }
        assert!(drawn_like(cx, Role::Article, "Ready to try:").is_empty(), "no build has a card of its own until one is opened");
        for gone in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(!drawn(cx, Role::Button, gone), "the compact card has no {gone}");
        }
        assert!(drawn(cx, Role::ListItem, "Ready to try: Chip takes the stale colour, Looks good"), "the answered build stays one line");
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
    fn a_press_on_a_row_opens_that_build_in_place_and_a_second_press_shuts_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let shot = id_of(&view, SHOT, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &shot, cx));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(shot.clone()));
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {SHOT}")), "the build is drawn whole under the rows");
        assert!(drawn(cx, Role::Button, "Looks good"));
        let full = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &full, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {FULL}")), "another row takes its place");
        assert_eq!(pick_of(&view, cx), Some(full.clone()));
        view.update(cx, |view, cx| view.ready_open("tool-edit", &full, cx));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), None, "a press on the open row shuts it");
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
        one(&view, cx);
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, BARE, cx)));
        one(&view, cx);
        view.read_with(cx, |view, cx| {
            let held = view.thread("tool-edit", cx).expect("a thread");
            assert_eq!(place_in(&held.items), None, "no build is left for the compact card");
        });
    }

    #[gpui::test]
    fn the_session_id_form_and_the_branch_form_each_open_a_build_in_place(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.open_artifact("tool-edit", "try:tool-edit", window, cx));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, BARE, cx)), "the Job's item opens its newest open build");
        let shot = id_of(&view, SHOT, cx);
        assert!(view.update_in(cx, |view, window, cx| view.open_named(&shot, window, cx)));
        redraw(cx);
        assert_eq!(pick_of(&view, cx), Some(shot), "a build's own id opens it");
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {SHOT}")));
        assert!(!view.update_in(cx, |view, window, cx| view.open_named("try:native/gone@0000", window, cx)), "a build the thread no longer holds opens nothing");
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
            (card.expect("the card"), place_in(&held.items).expect("a place"))
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
    fn needs_you_holds_one_item_for_the_job_with_a_plain_dismiss_and_no_answer_and_a_press_opens_the_card(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        redraw(cx);
        assert!(!drawn_like(cx, Role::Button, "Open Ready to try").is_empty(), "one item stands for the Job, however many builds");
        assert!(!drawn_like(cx, Role::Button, "Remove Ready to try").is_empty(), "a plain dismiss puts it away");
        assert!(drawn_like(cx, Role::Button, "Answer Ready to try").is_empty(), "no disabled Answer stands on it");
        press(cx, Role::Button, "Open Ready to try");
        assert_eq!(pick_of(&view, cx), Some(id_of(&view, BARE, cx)), "the newest open build is opened in place");
    }

    /// Whether a Refresh preview button is drawn inside the labelled element whose name starts with `inside`.
    fn refresh_in(cx: &mut gpui::VisualTestContext, inside: &str) -> bool {
        cx.update(|window, cx| {
            let id = window.window_handle().window_id();
            cx.global::<crate::labels::LabelBook>().drawn(id).iter().any(|one| one.role == Role::Button && one.name == "Refresh preview" && one.path.iter().any(|name| name.starts_with(inside)))
        })
    }

    #[gpui::test]
    fn refresh_preview_stands_on_the_card_head_while_a_listed_build_is_not_in_the_preview(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-edit", 1400.);
        assert!(drawn(cx, Role::Button, "Refresh preview"), "the card head offers it");
        assert!(refresh_in(cx, "Ready to try"), "inside the card");
    }

    #[gpui::test]
    fn refresh_preview_stands_beside_the_built_chip_of_a_build_not_in_the_preview(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let outside = id_of(&view, SHOT, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &outside, cx));
        redraw(cx);
        assert!(refresh_in(cx, &format!("Ready to try: {SHOT}")), "the build not in the preview offers Refresh beside its Built chip");
    }

    #[gpui::test]
    fn a_build_in_the_preview_has_no_refresh_beside_its_built_chip(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit", 1400.);
        let inside = id_of(&view, FULL, cx);
        view.update(cx, |view, cx| view.ready_open("tool-edit", &inside, cx));
        redraw(cx);
        assert!(!refresh_in(cx, &format!("Ready to try: {FULL}")), "nothing to refresh");
    }

    /// Fleet's one row for the Job, as live Fleet sends it: `try:<session>`, source ready_to_try, act message, in To review with
    /// the band offering the review. Its focus view is the newest open build with its own answers, never a question's deck.
    fn the_focus_view_of_the_ready_row(cx: &mut gpui::TestAppContext, wide: f32) {
        let (view, cx) = on_screen(cx, "tool-edit", wide);
        view.update_in(cx, |view, window, cx| view.set_needs(true, Some("item:try:tool-edit".to_string()), window, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {BARE}")), "the newest open build stands whole at {wide}");
        for name in ["Looks good", "Needs rework", "Remind me"] {
            assert!(drawn(cx, Role::Button, name), "{name} is on it at {wide}");
        }
        assert!(drawn(cx, Role::Button, "Back"), "and the way back to the list");
        assert!(drawn_like(cx, Role::Button, "Answer").is_empty(), "no Answer button for a ready row at {wide}");
        assert!(drawn_like(cx, Role::Group, "Answers").is_empty(), "and no deck of options at {wide}");
    }

    #[gpui::test]
    fn the_ready_rows_focus_view_is_the_newest_open_build_at_1400(cx: &mut gpui::TestAppContext) {
        the_focus_view_of_the_ready_row(cx, 1400.);
    }

    #[gpui::test]
    fn the_ready_rows_focus_view_is_the_newest_open_build_at_820(cx: &mut gpui::TestAppContext) {
        the_focus_view_of_the_ready_row(cx, 820.);
    }
}
