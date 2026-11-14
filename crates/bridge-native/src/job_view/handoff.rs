//! Fleet's handoff card: a Job past its context limit asks whether it carries on in a new Job
//! (`docs/concepts/session/a-handed-off-job.md`). One card, drawn in the thread from the `handoff` row
//! and in Needs you in place of its `handoff:<row>` item. A press answers the item with its `choice`;
//! once answered the card is one quiet line.

use std::rc::Rc;

use gpui::{canvas, div, prelude::*, AnyElement, Context, FontWeight, KeyDownEvent, Role, SharedString, Window};
use ipc::{handoff_figure, AnswerWaiting, HandoffPick, SessionId, SessionRow, ThreadItem, HANDOFF_REMIND, HANDOFF_STAY};

use super::acts::{kbd, refusal, Answer};
use super::rows::{glyph, mono, Size};
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

/// What the card asks, as Fleet's item says it.
pub(super) const ASKS: &str = "Carry on in a new Job?";
/// The waiting item's id, from its row's.
pub(super) const ITEM_PREFIX: &str = "handoff:";
/// How many lines of the brief show before Show all, and about how many the opened well holds.
const FOLDED_LINES: f32 = 4.;
const OPEN_LINES: f32 = 10.;
/// Under this width the head puts the question on a line of its own, and the card draws its presses closer.
const NARROW: f32 = 420.;

/// A handoff row's facts, as the card draws them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Card {
    pub id: String,
    pub tokens: u64,
    pub limit: u64,
    pub brief: String,
    pub remind_at: Vec<u64>,
    pub answer: Option<HandoffPick>,
}

impl Card {
    pub(super) fn of(row: &SessionRow) -> Option<Card> {
        match row {
            SessionRow::Handoff { id, tokens, limit, brief, remind_at, answer, .. } => {
                Some(Card { id: id.clone(), tokens: *tokens, limit: *limit, brief: brief.clone(), remind_at: remind_at.clone(), answer: *answer })
            }
            _ => None,
        }
    }

    /// Its six options in `choice` order, each as (what the button says, the name it is read by).
    fn options(&self) -> Vec<(String, String)> {
        let picks = [("Carry on", ipc::HANDOFF_YES), ("Stay here", HANDOFF_STAY), ("Don't ask again", ipc::HANDOFF_QUIET)];
        picks
            .into_iter()
            .map(|(said, name)| (said.to_string(), name.to_string()))
            .chain(self.remind_at.iter().map(|tokens| (handoff_figure(*tokens), format!("{HANDOFF_REMIND} {}", handoff_figure(*tokens)))))
            .collect()
    }
}

/// What a `choice` picks, as Fleet reads it: carry on, stay, don't ask again, then each figure.
pub(super) fn pick_of(choice: usize, remind_at: &[u64]) -> Option<HandoffPick> {
    match choice {
        0 => Some(HandoffPick::CarryOn),
        1 => Some(HandoffPick::Stay),
        2 => Some(HandoffPick::Quiet),
        more => remind_at.get(more - 3).map(|tokens| HandoffPick::RemindAt { tokens: *tokens }),
    }
}

/// The option a bare digit presses, `1` the first: none past the card's `count`.
pub(super) fn choice_of_key(key: &str, count: usize) -> Option<usize> {
    let n = key.parse::<usize>().ok().filter(|_| key.len() == 1)?;
    (1..=count).contains(&n).then(|| n - 1)
}

/// The quiet line an answered card becomes.
pub(super) fn answered_line(tokens: u64, pick: HandoffPick) -> String {
    let said = match pick {
        HandoffPick::CarryOn => "Carried on in a new Job".to_string(),
        HandoffPick::Stay => "Stayed here".to_string(),
        HandoffPick::Quiet => "Stayed here, never asks again".to_string(),
        HandoffPick::RemindAt { tokens } => format!("Stayed here, asks again at {}", handoff_figure(tokens)),
    };
    format!("Context {} \u{b7} {said}", handoff_figure(tokens))
}

/// Where the card is drawn: in the thread at its item, or in Needs you.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Place {
    Thread(usize),
    Needs,
}

impl JobView {
    /// The handoff row `row` of Job `job`, read off its thread.
    pub(super) fn handoff_of(&self, job: &str, row: &str, cx: &gpui::App) -> Option<Card> {
        self.thread(job, cx)?.items.iter().find_map(|item| match item {
            ThreadItem::Row { row: found } if found.id() == row => Card::of(found),
            _ => None,
        })
    }

    /// The card, or once answered its line. A press here or in Needs you answers the item.
    pub(super) fn handoff_card(&self, card: &Card, place: Place, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = format!("{ITEM_PREFIX}{}", card.id);
        let picked = match page.cards.answers.get(&key) {
            Some(Answer::Handoff(pick)) => Some(*pick),
            _ => None,
        };
        if let Some(pick) = picked.or(card.answer) {
            let said = answered_line(card.tokens, pick);
            return labelled(
                div()
                    .id(SharedString::from(format!("handoff-answered:{}", card.id)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(glyph(look, "layers", Size::Small, look.color(token::FG_MUTED)))
                    .child(div().flex_1().min_w_0().child(said.clone())),
                Role::Status,
                said,
            )
            .into_any_element();
        }
        let hue = look.color(token::STATUS_ESCALATED);
        let row_id = card.id.clone();
        let options = card.options();
        let opened = page.cards.opened.contains(&format!("{key}:brief"));
        let line = look.length(token::LEADING_SM);
        let folded = line * FOLDED_LINES;
        // The brief's whole height and the head's width as last drawn: a draw that changes either measures again.
        let side = if place == Place::Needs { "needs" } else { "thread" };
        let cell = |part: &str| page.cards.measured.borrow_mut().entry(format!("{key}:{side}:{part}")).or_default().clone();
        let (height, width) = (cell("brief"), cell("head"));
        let list = match place {
            Place::Thread(ix) => Some((page.list.clone(), page.entry(ix))),
            Place::Needs => None,
        };
        let tall = height.get() > folded + look.length(token::BORDER_WIDTH);
        let narrow = width.get() > gpui::px(0.) && width.get() < gpui::px(NARROW);
        let sunken = look.color(token::BG_SUNKEN);
        let words = div().relative().w_full().child(card.brief.trim_end().to_string()).child(measure(height, list.clone(), |bounds| bounds.size.height));
        let well = div()
            .id(SharedString::from(format!("handoff-brief:{}", card.id)))
            .relative()
            .w_full()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(sunken)
            .text_size(look.length(token::TEXT_SM))
            .line_height(line)
            .text_color(look.color(token::FG_DEFAULT));
        let well = if opened {
            // Opened, it scrolls inside its well, so the picks under it stay where they are.
            // Narrow, the card is taller than the thread: the well keeps four lines and scrolls, so the head stays in view.
            let lines = if narrow { FOLDED_LINES } else { OPEN_LINES };
            well.child(div().id(SharedString::from(format!("handoff-brief-scroll:{}", card.id))).max_h(line * lines).overflow_y_scroll().child(words))
        } else {
            well.child(div().max_h(folded).overflow_hidden().child(words)).when(tall, |well| {
                let clear = sunken.opacity(0.);
                well.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom(look.length(token::SPACE_2))
                        .h(line * 1.5)
                        .bg(gpui::linear_gradient(180., gpui::linear_color_stop(clear, 0.), gpui::linear_color_stop(sunken, 1.))),
                )
            })
        };
        let well = labelled(well, Role::Region, "Brief");
        let show_all = (tall || opened).then(|| {
            let said = if opened { "Show less" } else { "Show all" };
            let row = row_id.clone();
            labelled(
                pressed(self.handoff_press(format!("handoff-whole:{}", card.id), said.into(), Some("B"), Tone::Words, false, look), cx, move |this, _, cx| this.flip_handoff_brief(&row, place, cx)),
                Role::Button,
                said,
            )
        });
        let press = |at: usize, tone: Tone| {
            let (said, name) = options[at].clone();
            let row = row_id.clone();
            // Narrow, the presses drop their key caps so two share a row; 1 to 6 still press them.
            let cap = (!narrow).then(|| SharedString::from((at + 1).to_string()));
            labelled(
                pressed(self.handoff_press(format!("handoff-pick:{}:{at}", card.id), said.into(), cap.as_ref().map(|cap| cap.as_ref()), tone, narrow, look), cx, move |this, window, cx| {
                    this.answer_handoff(&row, at, place, window, cx)
                }),
                Role::Button,
                name,
            )
            .into_any_element()
        };
        let picks = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(press(0, Tone::Primary))
            .child(press(1, Tone::Plain))
            .child(press(2, Tone::Plain));
        let remind = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(div().flex_none().whitespace_nowrap().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(HANDOFF_REMIND))
            .children((3..options.len()).map(|at| press(at, Tone::Plain)));
        let answers = super::acts::gated(
            div().id(SharedString::from(format!("handoff-answers:{}", card.id))).flex().flex_col().gap(look.length(token::SPACE_2)).child(picks).child(remind),
            self.off_shown_to("/sessions/waiting/answer", cx),
        );
        let tile = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(look.length(token::H_CONTROL))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::STATUS_ESCALATED_BG))
            .child(glyph(look, "layers", Size::Mark, hue));
        let figure = div()
            .flex_none()
            .whitespace_nowrap()
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XL))
            .line_height(look.length(token::LEADING_XL))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(hue)
            .child(handoff_figure(card.tokens));
        let asks = div()
            .flex_1()
            .min_w_0()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(look.color(token::FG_DEFAULT))
            .child(ASKS);
        let limit = div()
            .flex_none()
            .ml_auto()
            .whitespace_nowrap()
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(format!("limit {}", handoff_figure(card.limit)));
        let row = || div().w_full().flex().items_center().gap(look.length(token::SPACE_3));
        // Narrow, the figure and the limit take the first line and the question its own under them.
        let head = if narrow {
            div().relative().flex().flex_col().gap(look.length(token::SPACE_2)).child(row().child(tile).child(figure).child(limit)).child(asks)
        } else {
            row().relative().child(tile).child(figure).child(asks).child(limit)
        }
        .child(measure(width, list, |bounds| bounds.size.width));
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::PAD_CARD))
            .child(head)
            .child(well)
            .children(show_all.map(|press| div().flex().child(press)))
            .child(answers)
            .children(page.cards.said.get(&key).map(|why| refusal(look, &key, why.clone())));
        let row = row_id.clone();
        let frame = div()
            .id(SharedString::from(match place {
                Place::Thread(_) => format!("card:{key}"),
                Place::Needs => format!("needs-{key}"),
            }))
            .w_full()
            .flex()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
            .child(div().flex_none().w(look.length(token::BORDER_WIDTH) * 3.).bg(hue))
            .child(body)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let key = &event.keystroke;
                if key.modifiers.modified() || this.handoff_typing(window, cx) {
                    return;
                }
                if key.key == "b" {
                    cx.stop_propagation();
                    this.flip_handoff_brief(&row, place, cx);
                } else if let Some(at) = choice_of_key(&key.key, 3 + this.handoff_figures(&row, cx)) {
                    cx.stop_propagation();
                    this.answer_handoff(&row, at, place, window, cx);
                }
            }));
        // In the thread the card holds the keys once pressed; in Needs you the panel holds them.
        let frame = match place {
            Place::Thread(_) => {
                let spot = self.spot(page, &format!("card:{key}"), cx);
                let focus = spot.focus.clone();
                frame.track_focus(&spot.focus).focus_visible(|style| style.border_color(look.color(token::ACCENT))).on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |_, _, window, cx| {
                        if !focus.contains_focused(window, cx) {
                            window.focus(&focus, cx);
                        }
                    }),
                )
            }
            Place::Needs => frame,
        };
        labelled(frame, Role::Article, ASKS).into_any_element()
    }

    /// One press of the card, a tab stop that Enter and Space press, with its key on it.
    fn handoff_press(&self, id: String, said: SharedString, cap: Option<&str>, tone: Tone, compact: bool, look: &Look) -> gpui::Stateful<gpui::Div> {
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let hue = look.color(token::STATUS_ESCALATED);
        let (ground, hover, ink, edge) = match tone {
            Tone::Primary => (hue, hue.opacity(0.85), look.color(token::FG_INVERSE), hue),
            Tone::Plain => (look.color(token::BG_RAISED), look.color(token::BG_HOVER), look.color(token::FG_DEFAULT), look.color(token::BORDER_STRONG)),
            Tone::Words => (gpui::transparent_black(), look.color(token::BG_HOVER), look.color(token::ACCENT), gpui::transparent_black()),
        };
        let cap = cap.map(|cap| {
            let cap = kbd(look, cap);
            if tone == Tone::Primary { cap.bg(ink.opacity(0.16)).border_color(ink.opacity(0.5)).text_color(ink) } else { cap }
        });
        let press = div()
            .id(SharedString::from(id))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .whitespace_nowrap()
            .h(look.length(if tone == Tone::Words || compact { token::H_CONTROL_SM } else { token::H_CONTROL }))
            .px(look.length(if tone == Tone::Words { token::SPACE_1 } else { token::SPACE_3 }))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(ground)
            .text_color(ink)
            .text_size(look.length(if tone == Tone::Words { token::TEXT_XS } else { token::TEXT_SM }))
            .font_weight(FontWeight(look.number(if tone == Tone::Primary { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.border_color(ring))
            .child(said)
            .children(cap);
        press
    }

    /// How many figures card `row` offers.
    fn handoff_figures(&self, row: &str, cx: &gpui::App) -> usize {
        self.shown.as_ref().and_then(|job| self.handoff_of(job, row, cx)).map_or(0, |card| card.remind_at.len())
    }

    /// The caret is in a box: a digit or B is a letter typed there.
    fn handoff_typing(&self, window: &Window, cx: &gpui::App) -> bool {
        let Some(id) = self.shown.as_ref() else { return false };
        self.pages.get(id).is_some_and(|page| page.cards.boxes.values().any(|area| gpui::Focusable::focus_handle(area.read(cx), cx).contains_focused(window, cx)))
    }

    /// B, or Show all: the brief opens in its well, or folds again.
    pub(super) fn flip_handoff_brief(&mut self, row: &str, place: Place, cx: &mut Context<Self>) {
        let key = format!("{ITEM_PREFIX}{row}:brief");
        if let Some(cards) = self.cards_mut() {
            if !cards.opened.remove(&key) {
                cards.opened.insert(key);
            }
        }
        self.handoff_changed(place, cx);
    }

    fn handoff_changed(&mut self, place: Place, cx: &mut Context<Self>) {
        match place {
            Place::Thread(ix) => self.card_changed(ix, cx),
            Place::Needs => self.card_changed(super::acts::SHEET, cx),
        }
    }

    /// A pick: the card reads it at once and leaves Needs you, and Fleet is told the `choice`. A refusal puts
    /// both back with Fleet's reason on the card.
    pub(super) fn answer_handoff(&mut self, row: &str, choice: usize, place: Place, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(card) = self.handoff_of(&job, row, cx) else { return };
        let Some(pick) = pick_of(choice, &card.remind_at) else { return };
        if card.answer.is_some() {
            return;
        }
        let key = format!("{ITEM_PREFIX}{row}");
        if let Some(page) = self.pages.get_mut(&job) {
            page.cards.answers.insert(key.clone(), Answer::Handoff(pick));
            page.cards.said.remove(&key);
            page.dock.hidden.insert(key.clone());
            if page.dock.detail.as_deref() == Some(&format!("item:{key}")) {
                page.dock.detail = None;
            }
        }
        self.want_needs(&job, cx);
        self.handoff_changed(place, cx);
        let body = ipc::encode(&AnswerWaiting { session_id: SessionId::carried(job.to_string()), item_id: key.clone(), choice: u32::try_from(choice).ok(), text: None, mode: None, fixed: None })
            .map_err(|why| format!("{why:?}"));
        let back = job.clone();
        self.post_with(
            job,
            "/sessions/waiting/answer",
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back) {
                    page.cards.answers.remove(&key);
                    page.cards.said.insert(key.clone(), why);
                    page.dock.hidden.remove(&key);
                    page.list.remeasure();
                }
                cx.notify();
            },
            window,
            cx,
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Primary,
    Plain,
    Words,
}

/// A canvas laid over a part, keeping `read` of its bounds in `seen`. A change draws again, and in the thread
/// measures the card's item again, since what the card shows depends on it.
fn measure(seen: Rc<std::cell::Cell<gpui::Pixels>>, list: Option<(gpui::ListState, usize)>, read: fn(gpui::Bounds<gpui::Pixels>) -> gpui::Pixels) -> gpui::Canvas<()> {
    canvas(
        move |bounds, window, cx| {
            let now = read(bounds);
            if seen.get() != now {
                seen.set(now);
                if let Some((list, at)) = list {
                    cx.defer(move |_| list.remeasure_items(at..at + 1));
                }
                window.refresh();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A press, on a click and on Enter or Space while it has the keys: the key goes no further.
fn pressed(element: gpui::Stateful<gpui::Div>, cx: &Context<JobView>, act: impl Fn(&mut JobView, &mut Window, &mut Context<JobView>) + 'static) -> gpui::Stateful<gpui::Div> {
    let act = Rc::new(act);
    let by_key = act.clone();
    element.on_click(cx.listener(move |this, _, window, cx| act(this, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
        let key = &event.keystroke;
        if matches!(key.key.as_str(), "enter" | "space") && !key.modifiers.modified() {
            cx.stop_propagation();
            by_key(this, window, cx);
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: &str = r#"{"kind":"handoff","id":"ho-1","at":"2026-11-13T09:00:00Z","tokens":312000,"limit":300000,"brief":"The fix is on branch fix/reader.","remind_at":[400000,450000,500000]}"#;

    #[test]
    fn a_handoff_row_decodes_from_fleets_json_into_its_card() {
        let row: SessionRow = serde_json::from_str(ROW).expect("decodes");
        let card = Card::of(&row).expect("a handoff row");
        assert_eq!(card, Card { id: "ho-1".into(), tokens: 312_000, limit: 300_000, brief: "The fix is on branch fix/reader.".into(), remind_at: vec![400_000, 450_000, 500_000], answer: None });
        let names: Vec<String> = card.options().into_iter().map(|(_, name)| name).collect();
        assert_eq!(names, ["Carry on in a new Job", "Stay here", "Don't ask again", "Remind at 400k", "Remind at 450k", "Remind at 500k"]);
        let answered: SessionRow = serde_json::from_str(&ROW.replace(r#""remind_at":[400000,450000,500000]"#, r#""remind_at":[400000,450000,500000],"answer":{"kind":"remind_at","tokens":450000}"#)).expect("decodes");
        assert_eq!(Card::of(&answered).and_then(|card| card.answer), Some(HandoffPick::RemindAt { tokens: 450_000 }));
    }

    #[test]
    fn keys_one_to_six_press_the_six_options_in_order() {
        let ladder = [400_000, 450_000, 500_000];
        let picked: Vec<Option<HandoffPick>> = ["1", "2", "3", "4", "5", "6"].iter().map(|key| choice_of_key(key, 6).and_then(|at| pick_of(at, &ladder))).collect();
        assert_eq!(
            picked,
            [
                Some(HandoffPick::CarryOn),
                Some(HandoffPick::Stay),
                Some(HandoffPick::Quiet),
                Some(HandoffPick::RemindAt { tokens: 400_000 }),
                Some(HandoffPick::RemindAt { tokens: 450_000 }),
                Some(HandoffPick::RemindAt { tokens: 500_000 }),
            ]
        );
        assert_eq!((1..=6).map(|n| choice_of_key(&n.to_string(), 6)).collect::<Vec<_>>(), (0..6).map(Some).collect::<Vec<_>>());
        for key in ["0", "7", "9", "b", "12", ""] {
            assert_eq!(choice_of_key(key, 6), None, "{key}");
        }
        assert_eq!(pick_of(6, &ladder), None);
    }

    #[test]
    fn an_answered_card_says_what_was_picked_in_one_line() {
        assert_eq!(answered_line(312_000, HandoffPick::CarryOn), "Context 312k \u{b7} Carried on in a new Job");
        assert_eq!(answered_line(312_000, HandoffPick::Stay), "Context 312k \u{b7} Stayed here");
        assert_eq!(answered_line(312_000, HandoffPick::Quiet), "Context 312k \u{b7} Stayed here, never asks again");
        assert_eq!(answered_line(312_000, HandoffPick::RemindAt { tokens: 450_000 }), "Context 312k \u{b7} Stayed here, asks again at 450k");
    }
}
