//! The four keys on a Job's waiting cards (`thread-keys.tsx`): yes, change, send back and no.
//!
//! A card registers what each key does while it is drawn, and drops out when it stops waiting.
//! `⌥↵` and `⌥X` open a dialog naming the cards that have the act, since they change something;
//! `⌥E` and `⌥R` open the target card's own box and ask nothing. The target is the card the person
//! last pressed while the thread or the panel has focus, else the newest card.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{anchored, deferred, div, prelude::*, Anchor, AnyElement, Context, FocusHandle, KeyDownEvent, Role, SharedString, Window};

use crate::labels::labelled;
use crate::look::{token, ActiveLook};

use super::acts::{button, kbd, Tone};
use super::JobView;

/// What a key does to a card.
pub(super) type Run = Rc<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Which {
    Yes,
    Change,
    SendBack,
    No,
}

impl Which {
    pub(super) fn act(self) -> &'static str {
        match self {
            Which::Yes => "thread_yes",
            Which::Change => "thread_change",
            Which::SendBack => "thread_send_back",
            Which::No => "thread_no",
        }
    }
}

/// One waiting card: where it sits, what it is called, and the acts it answers to.
#[derive(Clone)]
pub(super) struct Card {
    pub at: usize,
    pub kind: String,
    pub title: String,
    pub yes: Option<(String, Run)>,
    pub change: Option<(String, Run)>,
    pub send_back: Option<(String, Run)>,
    pub no: Option<(String, Run)>,
}

impl Card {
    fn act(&self, which: Which) -> Option<&(String, Run)> {
        match which {
            Which::Yes => self.yes.as_ref(),
            Which::Change => self.change.as_ref(),
            Which::SendBack => self.send_back.as_ref(),
            Which::No => self.no.as_ref(),
        }
    }

    fn same_as(&self, other: &Card) -> bool {
        let verbs = |card: &Card| [&card.yes, &card.change, &card.send_back, &card.no].map(|act| act.as_ref().map(|(verb, _)| verb.clone()));
        self.at == other.at && self.kind == other.kind && self.title == other.title && verbs(self) == verbs(other)
    }
}

/// The cards of one Job that wait, by key. Cards write to it while they are drawn.
#[derive(Default)]
pub(super) struct Cards {
    cards: RefCell<BTreeMap<String, Card>>,
    /// A card came or went since the last draw: the cards redraw once, so a chip shows on the target.
    changed: Cell<bool>,
    /// Cards a draw has not yet written again: a draw that does not reach them drops them.
    stale: RefCell<std::collections::BTreeSet<String>>,
    /// The focus and the edges of the rows a person can reach by key, by row key.
    spots: RefCell<std::collections::HashMap<String, Spot>>,
    /// The Judge's notes not yet acted on, in the order the plan drew them this frame.
    findings: RefCell<Vec<Found>>,
    /// What `⌘Enter` does in a card's open box, by card key (`DraftRow.tsx:309`).
    commits: RefCell<std::collections::HashMap<String, Run>>,
}

/// A Judge's note drawn and waiting: its row, and which note of which reading it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Found {
    pub key: String,
    pub reading: String,
    pub note: String,
}

/// A row a key reaches: its focus handle, kept between draws, and where it was last drawn.
#[derive(Clone)]
pub(super) struct Spot {
    pub focus: FocusHandle,
    pub bounds: Rc<Cell<gpui::Bounds<gpui::Pixels>>>,
    /// Drawn in the frame that is now being drawn: a row the list scrolled away is not.
    seen: Rc<Cell<bool>>,
}

impl Cards {
    /// Start a draw of every card whose key begins `prefix`.
    pub fn mark(&self, prefix: &str) {
        let keys: Vec<String> = self.cards.borrow().keys().filter(|key| key.starts_with(prefix)).cloned().collect();
        self.stale.borrow_mut().extend(keys);
    }

    /// End it: the cards the draw did not write are gone.
    pub fn sweep(&self) {
        let gone: Vec<String> = self.stale.take().into_iter().collect();
        for key in gone {
            self.set(&key, None);
        }
    }

    pub fn set(&self, key: &str, card: Option<Card>) {
        self.stale.borrow_mut().remove(key);
        let mut cards = self.cards.borrow_mut();
        match card {
            Some(card) => {
                if cards.get(key).is_some_and(|held| held.same_as(&card)) {
                    cards.insert(key.to_string(), card);
                } else {
                    cards.insert(key.to_string(), card);
                    self.changed.set(true);
                }
            }
            None => {
                self.commits.borrow_mut().remove(key);
                if cards.remove(key).is_some() {
                    self.changed.set(true);
                }
            }
        }
    }

    /// What `⌘Enter` in the card's open box sends, or nothing while the box holds nothing to send.
    pub fn commit(&self, key: &str, run: Option<Run>) {
        match run {
            Some(run) => self.commits.borrow_mut().insert(key.to_string(), run),
            None => self.commits.borrow_mut().remove(key),
        };
    }

    pub fn commit_of(&self, key: &str) -> Option<Run> {
        self.commits.borrow().get(key).cloned()
    }

    /// The focus handle and the drawn edges of the row `key`, made on its first draw.
    pub fn spot(&self, key: &str, cx: &gpui::App) -> Spot {
        let spot = self
            .spots
            .borrow_mut()
            .entry(key.to_string())
            .or_insert_with(|| Spot { focus: cx.focus_handle().tab_stop(true), bounds: Rc::default(), seen: Rc::default() })
            .clone();
        spot.seen.set(true);
        spot
    }

    /// Start a frame: say whether the focused row was left undrawn by the last one, then forget what was drawn.
    /// Keys and actions travel from the focused element, so a focus on a row the list scrolled away would
    /// leave them with nowhere to go.
    pub fn begin_frame(&self, window: &Window) -> bool {
        let spots = self.spots.borrow();
        let lost = spots.values().any(|spot| spot.focus.is_focused(window) && !spot.seen.get());
        for spot in spots.values() {
            spot.seen.set(false);
        }
        self.findings.borrow_mut().clear();
        lost
    }

    /// A note waiting was drawn: the keys that run down the notes find it in this order.
    pub fn found(&self, one: Found) {
        self.findings.borrow_mut().push(one);
    }

    pub fn found_all(&self) -> Vec<Found> {
        self.findings.borrow().clone()
    }

    /// Which of the drawn notes holds focus, by its place in the order.
    pub fn found_focused(&self, window: &Window) -> Option<usize> {
        let spots = self.spots.borrow();
        self.findings.borrow().iter().position(|one| spots.get(&one.key).is_some_and(|spot| spot.focus.is_focused(window)))
    }

    /// The waiting card that holds focus, or has something inside it that does: a card draws its root under
    /// one of these names.
    pub fn focused_card(&self, window: &Window, cx: &gpui::App) -> Option<String> {
        let spots = self.spots.borrow();
        self.cards.borrow().keys().find(|key| {
            [format!("card:{key}"), format!("plan-card:{key}")].iter().any(|name| spots.get(name).is_some_and(|spot| spot.focus.contains_focused(window, cx)))
        }).cloned()
    }

    pub fn spot_of(&self, key: &str) -> Option<Spot> {
        self.spots.borrow().get(key).cloned()
    }

    /// The block of words drawn under both points, by the id of its row: what a selection lies in.
    pub fn block_at(&self, a: gpui::Point<gpui::Pixels>, b: gpui::Point<gpui::Pixels>) -> Option<String> {
        self.spots
            .borrow()
            .iter()
            .filter(|(key, spot)| key.starts_with("block:") && spot.seen.get())
            .find(|(_, spot)| spot.bounds.get().contains(&a) && spot.bounds.get().contains(&b))
            .map(|(key, _)| key["block:".len()..].to_string())
    }

    /// Where the block of words under the id of its row was last drawn.
    pub fn block_origin(&self, row: &str) -> Option<gpui::Point<gpui::Pixels>> {
        self.spots.borrow().get(&format!("block:{row}")).map(|spot| spot.bounds.get().origin)
    }

    pub fn take_changed(&self) -> bool {
        self.changed.replace(false)
    }

    /// The cards that answer to `which`, in the order they are drawn.
    fn having(&self, which: Which) -> Vec<(String, Card)> {
        let mut found: Vec<(String, Card)> = self.cards.borrow().iter().filter(|(_, card)| card.act(which).is_some()).map(|(key, card)| (key.clone(), card.clone())).collect();
        found.sort_by_key(|(_, card)| card.at);
        found
    }

    fn newest(&self) -> Option<String> {
        self.cards.borrow().iter().max_by_key(|(_, card)| card.at).map(|(key, _)| key.clone())
    }

    /// The card the keys act on: `aimed` when it still waits, else the newest.
    pub fn target_of(&self, aimed: Option<&str>) -> Option<String> {
        aimed.filter(|key| self.holds(key)).map(str::to_string).or_else(|| self.newest())
    }

    fn holds(&self, key: &str) -> bool {
        self.cards.borrow().contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<Card> {
        self.cards.borrow().get(key).cloned()
    }
}

/// The dialog `⌥↵` and `⌥X` open: which of the waiting cards the act goes to.
pub(super) struct Confirm {
    which: Which,
    rows: Vec<(String, String)>,
    picked: usize,
    back_to: Option<FocusHandle>,
}

impl JobView {
    fn cards_of(&self, id: &str) -> Option<Rc<Cards>> {
        self.pages.get(id).map(|page| page.keys.clone())
    }

    /// The card a key acts on: the one that holds focus, else the newest.
    fn key_target(&self, id: &str, window: &Window, cx: &gpui::App) -> Option<String> {
        let cards = self.cards_of(id)?;
        // A card with focus in it is the target; the Needs you panel keeps the one answered last; anywhere else it is the newest.
        if let Some(held) = self.pages.get(id).and_then(|page| page.keys.focused_card(window, cx)) {
            return Some(held);
        }
        let aimed = self.pages.get(id).and_then(|page| page.cards.target.clone()).filter(|_| self.dock_focus.contains_focused(window, cx));
        cards.target_of(aimed.as_deref())
    }

    /// Whether some card has a yes: the message box then leaves `⌥↵` to it.
    pub(super) fn yes_waits(&self) -> bool {
        self.shown.as_ref().and_then(|id| self.cards_of(id)).is_some_and(|cards| !cards.having(Which::Yes).is_empty())
    }

    pub(super) fn thread_key(&mut self, which: Which, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(cards) = self.cards_of(&id) else { return };
        if self.confirm.is_some() {
            if which == Which::Yes {
                self.confirm_now(window, cx);
            }
            return;
        }
        match which {
            Which::Yes | Which::No => {
                let having = cards.having(which);
                if having.is_empty() {
                    cx.propagate();
                    return;
                }
                let target = self.key_target(&id, window, cx);
                let picked = target.and_then(|key| having.iter().position(|(held, _)| *held == key)).unwrap_or(having.len() - 1);
                let rows = having.iter().map(|(key, card)| (key.clone(), format!("{} \u{b7} {}", card.kind, card.title))).collect();
                self.confirm = Some(Confirm { which, rows, picked, back_to: window.focused(cx) });
                window.focus(&self.confirm_focus, cx);
                cx.notify();
            }
            Which::Change | Which::SendBack => {
                let Some(card) = self.key_target(&id, window, cx).and_then(|key| cards.get(&key)) else { return };
                if let Some((_, run)) = card.act(which) {
                    run(self, window, cx);
                }
            }
        }
    }

    fn close_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Confirm> {
        let confirm = self.confirm.take()?;
        match &confirm.back_to {
            Some(handle) => window.focus(handle, cx),
            None => window.focus(&self.focus, cx),
        }
        cx.notify();
        Some(confirm)
    }

    fn confirm_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(confirm) = self.close_confirm(window, cx) else { return };
        let Some((key, _)) = confirm.rows.get(confirm.picked) else { return };
        let run = self.cards_of(&id).and_then(|cards| cards.get(key)).and_then(|card| card.act(confirm.which).map(|(_, run)| run.clone()));
        if let Some(run) = run {
            run(self, window, cx);
        }
    }

    fn confirm_keys(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(confirm) = self.confirm.as_mut() else { return };
        let key = event.keystroke.key.as_str();
        match key {
            "escape" => {
                self.close_confirm(window, cx);
            }
            "enter" => self.confirm_now(window, cx),
            "tab" => {
                // Tab goes round the dialog: its list, Cancel, then the verb.
                let order = [self.confirm_focus.clone(), self.confirm_stops[0].clone(), self.confirm_stops[1].clone()];
                super::pop::trap_tab(&order, event.keystroke.modifiers.shift, window, cx);
                return;
            }
            "up" => confirm.picked = confirm.picked.saturating_sub(1),
            "down" => confirm.picked = (confirm.picked + 1).min(confirm.rows.len().saturating_sub(1)),
            digit if digit.len() == 1 && digit.chars().all(|c| c.is_ascii_digit()) && !event.keystroke.modifiers.alt => {
                let n = digit.parse::<usize>().unwrap_or(0);
                if (1..=confirm.rows.len().min(9)).contains(&n) {
                    confirm.picked = n - 1;
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub(super) fn render_confirm(&self, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let confirm = self.confirm.as_ref()?;
        let look = cx.look();
        let verb = self.shown.as_ref().and_then(|id| self.cards_of(id)).and_then(|cards| cards.get(&confirm.rows.get(confirm.picked)?.0)).and_then(|card| card.act(confirm.which).map(|(verb, _)| verb.clone())).unwrap_or_default();
        let rows = confirm.rows.iter().enumerate().map(|(at, (_, title))| {
            labelled(
                div()
                    .id(SharedString::from(format!("confirm-row:{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(if at == confirm.picked { look.color(token::ACCENT) } else { gpui::transparent_black() })
                    .bg(if at == confirm.picked { look.color(token::ACCENT_FAINT) } else { gpui::transparent_black() })
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(if at < 9 { kbd(look, &format!("{}", at + 1)).into_any_element() } else { div().into_any_element() })
                    .child(div().min_w_0().child(SharedString::from(title.clone())))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(confirm) = this.confirm.as_mut() {
                            confirm.picked = at;
                        }
                        cx.notify();
                    })),
                Role::RadioButton,
                title.clone(),
            )
        });
        let title = format!("{verb} which?");
        // The scrim is the whole window's, as Electron's body portal is, not the Job's room alone.
        let dialog =
            div()
                .id("thread-confirm-scrim")
                .w(crate::room::of(window, cx).size.width)
                .h(crate::room::of(window, cx).size.height)
                .flex()
                .items_center()
                .justify_center()
                .bg(look.color(token::BG_BASE).opacity(0.6))
                .occlude()
                .child(labelled(
                    div()
                        .id("thread-confirm")
                        .track_focus(&self.confirm_focus)
                        .on_key_down(cx.listener(Self::confirm_keys))
                        .w(look.length(token::SPACE_12) * 7.)
                        .max_w_full()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_4))
                        .p(look.length(token::SPACE_6))
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_OVERLAY))
                        .shadow(look.shadows(token::SHADOW_OVERLAY))
                        .child(div().text_size(look.length(token::TEXT_BASE)).font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(title.clone())))
                        .child(labelled(div().id("confirm-cards").flex().flex_col().gap(look.length(token::SPACE_1)).children(rows), Role::RadioGroup, "Waiting cards"))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap(look.length(token::SPACE_2))
                                .child(labelled(
                                    button(look, "confirm-cancel", "Cancel", Tone::Ghost, false, None, true).track_focus(&self.confirm_stops[0]).on_click(cx.listener(|this, _, window, cx| {
                                        this.close_confirm(window, cx);
                                    })),
                                    Role::Button,
                                    "Cancel",
                                ))
                                .child(labelled(
                                    button(look, "confirm-go", &verb, Tone::Primary, false, None, true).track_focus(&self.confirm_stops[1]).on_click(cx.listener(|this, _, window, cx| this.confirm_now(window, cx))),
                                    Role::Button,
                                    verb.clone(),
                                )),
                        ),
                    Role::Dialog,
                    title,
                ));
        Some(deferred(anchored().anchor(Anchor::TopLeft).position(crate::room::of(window, cx).origin).child(dialog)).with_priority(3).into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(at: usize, kind: &str, yes: bool) -> Card {
        let run: Run = Rc::new(|_, _, _| {});
        Card { at, kind: kind.into(), title: kind.into(), yes: yes.then(|| ("Yes".into(), run)), change: None, send_back: None, no: None }
    }

    #[test]
    fn the_newest_card_is_the_target_until_one_is_aimed_at() {
        let cards = Cards::default();
        cards.set("plan", Some(card(3, "Plan", true)));
        cards.set("draft", Some(card(1, "Draft", true)));
        assert_eq!(cards.target_of(None).as_deref(), Some("plan"));
        assert_eq!(cards.target_of(Some("draft")).as_deref(), Some("draft"));
        // A card that no longer waits is not aimed at.
        assert_eq!(cards.target_of(Some("gone")).as_deref(), Some("plan"));
    }

    #[test]
    fn the_cards_with_an_act_come_in_the_order_they_are_drawn() {
        let cards = Cards::default();
        cards.set("b", Some(card(5, "B", true)));
        cards.set("a", Some(card(2, "A", true)));
        cards.set("c", Some(card(9, "C", false)));
        let keys: Vec<String> = cards.having(Which::Yes).into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["a", "b"]);
    }

    #[test]
    fn a_draw_that_does_not_write_a_card_again_drops_it() {
        let cards = Cards::default();
        cards.set("question:1", Some(card(1, "Question", true)));
        cards.set("plan", Some(card(2, "Plan", true)));
        cards.mark("question:");
        cards.sweep();
        assert!(cards.get("question:1").is_none());
        assert!(cards.get("plan").is_some());
    }

    #[test]
    fn a_card_that_came_or_went_asks_for_one_more_draw() {
        let cards = Cards::default();
        assert!(!cards.take_changed());
        cards.set("plan", Some(card(1, "Plan", true)));
        assert!(cards.take_changed());
        // The same card written again changes nothing.
        cards.set("plan", Some(card(1, "Plan", true)));
        assert!(!cards.take_changed());
        cards.set("plan", None);
        assert!(cards.take_changed());
    }
}
