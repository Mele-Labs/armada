//! The menus a thread opens: a card's revisions, a Fleet notice's Teach and mute, a message's fork.
//! One is open at a time, over the thread, and a pick runs with the view that opened it.

use gpui::{AppContext as _, Anchor, Context, DismissEvent, Entity, FocusHandle, Focusable as _, Pixels, Point, SharedString, Subscription, Window};

use super::JobView;
use crate::dropdown::{Dropdown, Entry, Picked};

/// A menu that is open, and what listens to it.
pub(super) struct Pop {
    pub menu: Entity<Dropdown>,
    /// Who held the keys before the menu opened, which has them again once it shuts.
    back: Option<FocusHandle>,
    _picked: Subscription,
    _dismissed: Subscription,
}

/// Each entry id is a static string, so a menu of numbered choices draws from this table.
pub(super) const NUMBERED: [&str; 12] = ["n1", "n2", "n3", "n4", "n5", "n6", "n7", "n8", "n9", "n10", "n11", "n12"];

impl JobView {
    /// Open a menu at `at`. `pick` runs with the entry's id when one is chosen, and the menu shuts.
    pub(super) fn open_pop(
        &mut self,
        name: impl Into<SharedString>,
        entries: Vec<Entry>,
        at: Point<Pixels>,
        from_keys: bool,
        pick: impl Fn(&mut JobView, &'static str, &mut Window, &mut Context<JobView>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_pop_back(name, entries, at, from_keys, None, pick, window, cx);
    }

    /// As [`Self::open_pop`], with the keys on the menu while it is up and given back to `back` when it shuts.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn open_pop_back(
        &mut self,
        name: impl Into<SharedString>,
        entries: Vec<Entry>,
        at: Point<Pixels>,
        from_keys: bool,
        back: Option<FocusHandle>,
        pick: impl Fn(&mut JobView, &'static str, &mut Window, &mut Context<JobView>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = name.into();
        let menu = cx.new(|cx| Dropdown::new(name, entries, at, Anchor::TopLeft, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, keep } = picked;
            if !keep {
                if let Some(back) = this.pop.take().and_then(|pop| pop.back) {
                    window.focus(&back, cx);
                }
            }
            pick(this, id, window, cx);
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            if let Some(back) = this.pop.take().and_then(|pop| pop.back) {
                window.focus(&back, cx);
            }
            cx.notify();
        });
        if back.is_some() {
            let keys = menu.read(cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&keys, cx));
        }
        self.pop = Some(Pop { menu, back, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }
}
