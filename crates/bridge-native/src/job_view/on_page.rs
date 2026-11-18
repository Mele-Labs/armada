//! A Job's own card drawn on another screen's page: the Operator page draws a Job's Ready to try as the thread does, with its
//! own answers, from the Job view that holds the card's state. The Job must be the one the view shows, as the window's other
//! screens open a Job without a pick; until it is, the page draws its skeleton.

use chrono::Local;
use gpui::{AnyElement, App, Entity, Global, SharedString, WeakEntity};
use ipc::ThreadItem;

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

/// The Job the view shows, which is the one whose cards a page can draw.
pub fn shown_on_page(cx: &App) -> Option<SharedString> {
    view(cx)?.read(cx).shown.clone()
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
