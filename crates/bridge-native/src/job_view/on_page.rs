//! A Job's own card drawn on another screen's page: the Operator page draws a Job's Ready to try as the thread does, with its
//! own answers, from the Job view that holds the card's state. The Job must be the one the view shows, as the window's other
//! screens open a Job without a pick; until it is, the page draws its skeleton. The Operator's own thread and box are drawn
//! under the card the same way, from a second session the view holds beside the Job it shows (`beside.rs`).

use chrono::Local;
use gpui::{prelude::*, AnyElement, AnyView, App, Entity, Global, ListState, SharedString, WeakEntity, Window};
use ipc::ThreadItem;

use super::band_card::Act;
use super::ready_card::{Parts, Standing};
use super::JobView;
use crate::look::ActiveLook;

/// The Job view a page draws a Job's card from. Set by the Job view, as `OpenOver` is.
struct OnPage(WeakEntity<JobView>);

impl Global for OnPage {}

pub(super) fn offer(cx: &mut gpui::Context<JobView>) {
    let view = cx.entity().downgrade();
    cx.set_global(OnPage(view));
}

fn view(cx: &App) -> Option<Entity<JobView>> {
    cx.try_global::<OnPage>()?.0.upgrade()
}

/// **Job `job`'s Ready to try card `item` whole**, at `wide` points, with Looks good, Needs rework, Show me how, Remind me and
/// Refresh preview, answered as in the thread. `item` is a card's id or the Job's one row (`try:<session>`). None while the view
/// shows another Job, or the Job's thread is not read yet.
pub fn ready_on_page(job: &str, item: &str, wide: f32, cx: &mut App) -> Option<AnyElement> {
    let view = view(cx)?;
    if view.read(cx).shown.as_deref() != Some(job) {
        return None;
    }
    let look = cx.look().clone();
    view.update(cx, |view, cx| {
        let held = view.thread(job, cx)?;
        let (at, _) = super::ready_card::card_named(&held.items, item)?;
        let ThreadItem::ReadyToTry(card) = held.items.get(at)? else { return None };
        let page = view.pages.get(job)?;
        Some(view.ready_detail(card, super::acts::SHEET, wide, page, Local::now(), &look, cx))
    })
}

/// **1, 2 or 3 on the page answer Job `job`'s Ready to try card `item`** as its buttons do: Looks good, Needs rework, Show me
/// how. As on the thread, a build that no longer asks, is not in the preview, or has a box open for his words takes none.
/// Whether the key was the card's.
pub fn ready_key_on_page(job: &str, item: &str, digit: &str, window: &mut Window, cx: &mut App) -> bool {
    let act = match digit {
        "1" => Act::Approve,
        "2" => Act::Ask,
        "3" => Act::Open,
        _ => return false,
    };
    let Some(view) = view(cx) else { return false };
    if view.read(cx).shown.as_deref() != Some(job) {
        return false;
    }
    view.update(cx, |view, cx| {
        let found = {
            let (Some(held), Some(page)) = (view.thread(job, cx), view.pages.get(job)) else { return false };
            let Some((at, _)) = super::ready_card::card_named(&held.items, item) else { return false };
            let Some(ThreadItem::ReadyToTry(card)) = held.items.get(at) else { return false };
            let standing = Standing::of(card, page.cards.answers.get(&card.id));
            let noting = page.cards.modes.contains_key(&card.id);
            let shows = act != Act::Open || Parts::of(card).first_go().is_some();
            (standing.asks() && card.in_build && !noting && shows).then(|| card.clone())
        };
        let Some(card) = found else { return false };
        let run = view.ready_run(super::acts::SHEET, &card, Parts::of(&card));
        run(view, act, window, cx);
        true
    })
}

/// **The Operator's thread and box, drawn as a Job's thread and box are**, under the card on its page. `pane` is what the page
/// has of the Operator now. None until the Job view is made.
pub fn operator_thread_on_page(pane: super::OperatorPane, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    let view = view(cx)?;
    let drawn = view.update(cx, |view, cx| view.pin_beside(pane, window, cx));
    Some(AnyView::from(drawn).into_any_element())
}

/// The list the Operator's thread is drawn in, which the page measures to fold the card above it.
pub fn operator_list(cx: &App) -> Option<ListState> {
    let view = view(cx)?;
    let view = view.read(cx);
    let id = &view.beside.as_ref()?.id;
    Some(view.pages.get(id)?.list.clone())
}

/// Discuss: the Operator's box names the card, and takes the keys.
pub fn operator_tag(card: String, title: SharedString, window: &mut Window, cx: &mut App) {
    let Some(view) = view(cx) else { return };
    view.update(cx, |view, cx| {
        view.set_about(Some((card, title)), window, cx);
        view.on_beside(|view| view.composer.input.update(cx, |input, cx| input.focus(window, cx)));
    });
}

/// The talk about the card is over: the box names no card, and the words in it stay, kept for the page.
pub fn operator_untag(cx: &mut App) {
    let Some(view) = view(cx) else { return };
    view.update(cx, |view, cx| view.drop_about(cx));
}

/// The caret goes in the Operator's box.
pub fn operator_focus(window: &mut Window, cx: &mut App) {
    let Some(view) = view(cx) else { return };
    view.update(cx, |view, cx| {
        view.on_beside(|view| view.composer.input.update(cx, |input, cx| input.focus(window, cx)));
    });
}

/// Whether the Operator's box has the keys.
pub fn operator_has_keys(window: &Window, cx: &App) -> bool {
    view(cx).is_some_and(|view| view.read(cx).beside_has_keys(window, cx))
}

/// The words in the Operator's box.
pub fn operator_words(cx: &App) -> String {
    view(cx).map(|view| view.read(cx).beside_words(cx)).unwrap_or_default()
}

/// Fleet answered the Operator's send `key`: a refusal leaves it in the thread marked Not sent, with its Retry.
pub fn operator_answered(key: u64, said: Result<(), String>, cx: &mut App) {
    let Some(view) = view(cx) else { return };
    view.update(cx, |view, cx| view.operator_answered(key, said, cx));
}

/// The Operator's thread scrolled up to its last words, where it stops following its end. Whether it has said any.
pub fn operator_to_last_reply(cx: &mut App) -> bool {
    let Some(view) = view(cx) else { return false };
    view.update(cx, |view, cx| view.to_last_reply(cx))
}

/// What he has sent Job `job`, in its thread and not echoed yet, for tests.
#[cfg(test)]
pub fn sent_to_job(job: &str, cx: &App) -> Vec<String> {
    let Some(view) = view(cx) else { return Vec::new() };
    let view = view.read(cx);
    let thread = view.mock.get(job).into_iter().flat_map(|held| held.items.iter()).filter_map(|item| match item {
        ThreadItem::Row { row: ipc::SessionRow::Message { from: ipc::SessionVoice::You, text, .. } } => Some(text.clone()),
        _ => None,
    });
    let pending = view.pages.get(job).into_iter().flat_map(|page| page.pending.iter().map(|one| one.text.to_string()));
    thread.chain(pending).collect()
}

/// The Operator's sends its thread has not echoed yet, for tests.
#[cfg(test)]
pub fn operator_unechoed(cx: &App) -> Vec<String> {
    let Some(view) = view(cx) else { return Vec::new() };
    let view = view.read(cx);
    let Some(page) = view.beside.as_ref().and_then(|beside| view.pages.get(&beside.id)) else { return Vec::new() };
    page.pending.iter().map(|one| one.text.to_string()).collect()
}

/// The card the Operator's box names, for tests.
#[cfg(test)]
pub fn operator_tagged(cx: &App) -> Option<(String, SharedString)> {
    view(cx)?.read(cx).beside.as_ref()?.about.clone()
}

/// What the page's answers sent for Job builds on mock data, and whether a build's box for his words is open, for tests.
#[cfg(test)]
pub fn ready_sent_on_page(cx: &App) -> (Vec<ipc::AnswerReadyToTry>, Vec<String>) {
    let Some(view) = view(cx) else { return Default::default() };
    let view = view.read(cx);
    let noting = view.shown.as_ref().and_then(|job| view.pages.get(job)).map(|page| page.cards.modes.keys().cloned().collect()).unwrap_or_default();
    (view.mock_ready.clone(), noting)
}
