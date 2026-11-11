//! The toasts drawn: the stack in the window's bottom trailing corner, the history he opens from the
//! bell or ⌥⌘N, and the bell itself in the title bar's end group, with a dot while a toast is unseen.
//!
//! **A press on a toast is the page's own press.** The pick opens the item on the Operator's page and
//! says Yes there, Later puts it last there, so the page and the toast never disagree on what is
//! answered. A toast ends when Your turn no longer holds its item, wherever it was answered.
//!
//! **An urgent toast also posts a banner**, with the blocked call's sound, when no window of the app is
//! in front and `features.notifications` is on: the toast alone would wait unseen behind another app.
//!
//! **History is kept on this Mac**, beside the deck's columns, and only from the owner's Fleet: a walk
//! leaves it as it was.

use std::path::PathBuf;

use gpui::{
    div, prelude::*, AnyWindowHandle, App, Context, Entity, FocusHandle, FontWeight, Global, Hsla, KeyDownEvent, Role, SharedString,
    Stateful, Subscription, WeakEntity, Window,
};
use ipc::operator_page::{OperatorAnswer, OperatorTurnItem, OperatorTurnKind};
use ipc::CallTone;

use super::{Ended, FleetNotice, NoticeAct, Shelf, Toast, Urgency};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, ActiveLook, Look};
use crate::operator::face;
use crate::operator::page::{Arrived, OperatorPageView};
use crate::operator::screen::{Front, OperatorScreen};
use crate::operator::view::{kind_chip, press, Weight};
use crate::titlebar::parts::{glyph, icon_button, sans, tip};

/// A toast's width: its three presses on one line, keys and all.
const WIDTH: f32 = 400.;

/// The most toasts the history file keeps.
const KEPT: usize = 200;

/// The bell, for the title bar to draw in its end group.
pub struct BellSlot(pub gpui::AnyView);

impl Global for BellSlot {}

/// The toasts of the window in front, for the keys that work from anywhere.
struct Live(WeakEntity<Toasts>, AnyWindowHandle);

impl Global for Live {}

/// Fleet's states as the title bar last read them, drawn as urgent toasts above the Operator's.
#[derive(Default)]
struct FleetNotices(Vec<FleetNotice>);

impl Global for FleetNotices {}

const RESTART_TOOLTIP: &str = "Restart Fleet. Working Drones are adopted and cannot be redirected until they finish.";

/// The title bar read Fleet's state again: what it says now stands, and a state not shown before marks
/// the bell.
pub fn fleet(notices: Vec<FleetNotice>, cx: &mut App) {
    let was: Vec<&'static str> = cx.try_global::<FleetNotices>().map(|kept| kept.0.iter().map(|notice| notice.id).collect()).unwrap_or_default();
    let fresh = notices.iter().any(|notice| !was.contains(&notice.id));
    cx.set_global(FleetNotices(notices));
    if let Some(toasts) = cx.try_global::<Live>().and_then(|live| live.0.upgrade()) {
        toasts.update(cx, |toasts, cx| {
            toasts.unseen |= fresh;
            cx.notify();
        });
    }
}

/// ⌥⌘J and ⌥⌘N, from anywhere (`actions.toml`, scope `anywhere`).
pub fn bind(cx: &mut App) {
    cx.on_action(|key: &Act, cx: &mut App| {
        let history = match key.id {
            "toast_focus" => false,
            "toast_history" => true,
            _ => return cx.propagate(),
        };
        let Some(Live(toasts, handle)) = cx.try_global::<Live>().map(|live| Live(live.0.clone(), live.1)) else { return };
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(toasts) = toasts.upgrade() {
                    toasts.update(cx, |toasts, cx| if history { toasts.toggle_history(window, cx) } else { toasts.focus_newest(window, cx) });
                }
            });
        });
    });
}

pub struct Toasts {
    shelf: Shelf,
    page: Entity<OperatorPageView>,
    screen: WeakEntity<OperatorScreen>,
    history: bool,
    /// A toast raised since he last opened the history.
    unseen: bool,
    /// The newest toast's keys: whichever toast is at the bottom tracks it.
    focus: FocusHandle,
    history_focus: FocusHandle,
    /// Where the keys were before ⌥⌘J or the history took them.
    back: Option<FocusHandle>,
    /// The history file, on the owner's Fleet only.
    file: Option<PathBuf>,
    _subs: Vec<Subscription>,
}

impl Toasts {
    /// The toasts over the screen, and the bell in the title bar.
    pub fn mount(page: &Entity<OperatorPageView>, screen: WeakEntity<OperatorScreen>, window: &mut Window, cx: &mut App) -> Entity<Toasts> {
        let toasts = cx.new(|cx| Toasts::new(page.clone(), screen, window, cx));
        let bell = cx.new(|cx| Bell::new(&toasts, cx));
        cx.set_global(BellSlot(bell.into()));
        cx.set_global(Live(toasts.downgrade(), window.window_handle()));
        toasts
    }

    fn new(page: Entity<OperatorPageView>, screen: WeakEntity<OperatorScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let file = page.read(cx).fleet().and(history_file());
        let shelf = Shelf::kept(file.as_ref().map(read_history).unwrap_or_default());
        let subs = vec![
            cx.subscribe_in(&page, window, |this, _, arrived: &Arrived, window, cx| this.arrived(&arrived.0, window, cx)),
            cx.observe(&page, |this, page, cx| this.page_moved(&page, cx)),
        ];
        Toasts { shelf, page, screen, history: false, unseen: false, focus: cx.focus_handle(), history_focus: cx.focus_handle(), back: None, file, _subs: subs }
    }

    fn arrived(&mut self, items: &[OperatorTurnItem], window: &mut Window, cx: &mut Context<Self>) {
        let raised: Vec<OperatorTurnItem> = self.shelf.raise(items).into_iter().map(|toast| toast.item.clone()).collect();
        if raised.is_empty() {
            return;
        }
        self.unseen = true;
        let notifications = self.page.read(cx).fleet().is_some_and(|fleet| fleet.read(cx).held().notifications);
        if notifications && !window.is_window_active() {
            for item in raised.iter().filter(|item| item.urgency == Urgency::Urgent) {
                crate::banner::post(&item.id, &face::card_title(item), &sentence(item), crate::notify::sound_of(CallTone::Blocked));
            }
        }
        self.keep(cx);
    }

    /// Fleet's page moved: an open toast whose item has left Your turn was settled, here or anywhere.
    fn page_moved(&mut self, page: &Entity<OperatorPageView>, cx: &mut Context<Self>) {
        let page = page.read(cx);
        if !page.is_read() {
            return;
        }
        let waiting: Vec<String> = page.items().iter().map(|turn| turn.id.clone()).collect();
        let waiting: Vec<&str> = waiting.iter().map(String::as_str).collect();
        self.shelf.keep_open(&waiting);
        self.keep(cx);
    }

    /// Write the history, where it is kept, and draw again.
    fn keep(&mut self, cx: &mut Context<Self>) {
        if let Some(file) = &self.file {
            let mut lines: Vec<String> = self.shelf.history().take(KEPT).filter_map(line_of).collect();
            lines.reverse();
            if let Err(why) = std::fs::write(file, lines.join("\n")) {
                Note::warn("the toasts' history was not kept").field("why", why.to_string()).emit();
            }
        }
        cx.notify();
    }

    // -- presses -----------------------------------------------------------------------------------

    /// The pick: Yes to the item on the page, with the Operator's pick where it has one.
    fn pick(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |page, cx| {
            page.open_id(&id, cx);
            page.answer(OperatorAnswer::Yes, window, cx);
        });
        self.ended(&id, Ended::Settled, window, cx);
    }

    fn later(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |page, cx| {
            page.open_id(&id, cx);
            page.later(cx);
        });
        self.ended(&id, Ended::Later, window, cx);
    }

    fn close(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.ended(&id, Ended::Closed, window, cx);
    }

    /// The Operator's page in front, the item open on it.
    fn open_operator(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |page, cx| page.open_id(&id, cx));
        self.history = false;
        self.back = None;
        if let Some(screen) = self.screen.upgrade() {
            screen.update(cx, |screen, cx| screen.bring(Some(Front::Operator), window, cx));
        }
        cx.notify();
    }

    /// A toast went: the keys stay on the toasts while one is left, else go back where they were.
    fn ended(&mut self, id: &str, how: Ended, window: &mut Window, cx: &mut Context<Self>) {
        let had_keys = self.focus.is_focused(window);
        self.shelf.end(id, how);
        if had_keys && self.shelf.shown().is_empty() {
            self.give_back(window, cx);
        }
        self.keep(cx);
    }

    fn give_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(back) = self.back.take() {
            window.focus(&back, cx);
        }
    }

    pub fn focus_newest(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shelf.shown().is_empty() {
            return;
        }
        if !self.focus.is_focused(window) {
            self.back = window.focused(cx).filter(|was| was != &self.history_focus);
        }
        self.history = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub fn toggle_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.history = !self.history;
        if self.history {
            self.unseen = false;
            if !self.focus.is_focused(window) {
                self.back = window.focused(cx);
            }
            window.focus(&self.history_focus, cx);
        } else {
            self.give_back(window, cx);
        }
        cx.notify();
    }

    /// A, L and Esc on the newest toast.
    fn toast_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.modified() {
            return;
        }
        let Some(id) = self.shelf.shown().last().map(|toast| toast.item.id.clone()) else { return };
        match key.key.as_str() {
            "a" => self.pick(id, window, cx),
            "l" => self.later(id, window, cx),
            "escape" => self.close(id, window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    // -- drawn -------------------------------------------------------------------------------------

    fn toast(&self, toast: &Toast, newest: bool, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let item = &toast.item;
        let id = item.id.clone();
        let title = face::card_title(item);
        let pick_word: SharedString = match item.kind {
            OperatorTurnKind::Decision => face::pick_of(item).unwrap_or_else(|| "Yes".into()),
            OperatorTurnKind::GoAhead => "Yes".into(),
        };
        let muted = muted_line(item);
        let key = |word: &'static str| newest.then(|| SharedString::from(word));
        let pick = {
            let id = id.clone();
            labelled(
                press(SharedString::from(format!("toast-pick:{id}")), pick_word.clone(), None, key("A"), Weight::Pick, look)
                    .on_click(cx.listener(move |this, _, window, cx| this.pick(id.clone(), window, cx))),
                Role::Button,
                pick_word,
            )
        };
        let later = {
            let id = id.clone();
            labelled(
                press(SharedString::from(format!("toast-later:{id}")), "Later".into(), Some("clock"), key("L"), Weight::Bare, look)
                    .on_click(cx.listener(move |this, _, window, cx| this.later(id.clone(), window, cx))),
                Role::Button,
                "Later",
            )
        };
        let open = {
            let id = id.clone();
            labelled(
                press(SharedString::from(format!("toast-open:{id}")), "Open the Operator".into(), None, None, Weight::Bare, look)
                    .on_click(cx.listener(move |this, _, window, cx| this.open_operator(id.clone(), window, cx))),
                Role::Button,
                "Open the Operator",
            )
        };
        let close = {
            let id = id.clone();
            labelled(
                icon_button(SharedString::from(format!("toast-close:{id}")), "x", look.color(token::FG_SUBTLE), look)
                    .tooltip(tip("Close — Esc"))
                    .on_click(cx.listener(move |this, _, window, cx| this.close(id.clone(), window, cx))),
                Role::Button,
                format!("Close toast: {title}"),
            )
        };
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(glyph("bot", look.length(token::SPACE_4), look.color(token::OPERATOR)))
            .child(
                div()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(format!("Operator · {}", clock_of(item.raised_at.as_str()))),
            )
            .child(kind_chip(face::kind_of(item), look))
            .child(div().flex_1())
            .child(close);
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(head)
            .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title.clone()))
            .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).line_clamp(2).child(sentence(item)))
            .children(muted.map(|line| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).truncate().child(line)))
            .child(div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).pt(look.length(token::SPACE_1)).child(pick).child(later).child(open));
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let frame = div()
            .id(SharedString::from(format!("toast:{id}")))
            .w(gpui::px(WIDTH))
            .flex()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .font_family(sans(look))
            .child(div().flex_none().w(gpui::px(3.)).bg(edge_of(toast.urgency, look)))
            .child(body);
        let frame = if newest {
            frame
                .track_focus(&self.focus)
                .focus(move |style| style.border_color(ring))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.toast_key(event, window, cx)))
        } else {
            frame
        };
        labelled(frame, Role::Alert, format!("Toast: {title}"))
    }

    /// A state of Fleet: the urgent edge, what is wrong and what to do, its code, and its acts. It has no
    /// close: it goes when the state does, or, after a restart that did not come back, on Dismiss.
    fn fleet_toast(&self, notice: &FleetNotice, look: &Look) -> impl IntoElement {
        let edge = look.color(if notice.fault { token::STATUS_COMPLETED_FAILED } else { token::DEGRADED });
        let act = match notice.act {
            NoticeAct::None => None,
            NoticeAct::Restart { working, busy } => {
                let word: SharedString = if working { "Restarting Fleet" } else { "Restart Fleet" }.into();
                let button = press("toast-restart-fleet", word.clone(), Some("rotate-cw"), None, Weight::Pick, look).tooltip(tip(RESTART_TOOLTIP));
                let button = if busy { button.opacity(0.6) } else { button.on_click(|_, _, cx| crate::titlebar::restart_fleet(cx)) };
                Some(labelled(button, Role::Button, word).into_any_element())
            }
            NoticeAct::Dismiss => Some(
                labelled(
                    press("toast-dismiss-restart", "Dismiss".into(), None, None, Weight::Bare, look).on_click(|_, _, cx| crate::titlebar::dismiss_restart_failure(cx)),
                    Role::Button,
                    "Dismiss",
                )
                .into_any_element(),
            ),
        };
        let copy = labelled(
            press(SharedString::from(format!("toast-copy:{}", notice.id)), "Copy debug info".into(), Some("copy"), None, Weight::Bare, look)
                .on_click(|_, _, cx| crate::titlebar::copy_debug_info(cx)),
            Role::Button,
            "Copy debug info",
        );
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(glyph("server-off", look.length(token::SPACE_4), edge))
            .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Fleet"))
            .child(
                div()
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(edge)
                    .text_color(look.color(token::FG_INVERSE))
                    .font_family(crate::titlebar::parts::mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .child(notice.code),
            );
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(head)
            .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(notice.headline.clone()))
            .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(notice.next.clone()))
            .child(div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).pt(look.length(token::SPACE_1)).children(act).child(copy));
        labelled(
            div()
                .id(SharedString::from(format!("toast:{}", notice.id)))
                .w(gpui::px(WIDTH))
                .flex()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG) * 1.5)
                .border(look.length(token::BORDER_WIDTH))
                .border_color(edge.opacity(0.6))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .font_family(sans(look))
                .child(div().flex_none().w(gpui::px(3.)).bg(edge))
                .child(body),
            Role::Alert,
            format!("Toast: {}", notice.headline),
        )
    }

    fn history_panel(&self, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let rows = self.shelf.history().map(|toast| {
            let id = toast.item.id.clone();
            let title = face::card_title(&toast.item);
            let said = match toast.ended {
                None => "Open",
                Some(Ended::Settled) => "Settled",
                Some(Ended::Closed) => "Closed",
                Some(Ended::Later) => "Later",
            };
            let row = div()
                .id(SharedString::from(format!("toast-history:{id}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .h(look.length(token::H_CONTROL_LG))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(div().flex_none().w(gpui::px(3.)).h(look.length(token::SPACE_4)).rounded_full().bg(edge_of(toast.urgency, look)))
                .child(kind_chip(face::kind_of(&toast.item), look))
                .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(title.clone()))
                .child(div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(said))
                .child(div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(clock_of(toast.item.raised_at.as_str())))
                .on_click(cx.listener(move |this, _, window, cx| this.open_operator(id.clone(), window, cx)));
            labelled(row, Role::ListItem, format!("{title}, {said}"))
        });
        let panel = div()
            .id("toast-history")
            .track_focus(&self.history_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    cx.stop_propagation();
                    this.toggle_history(window, cx);
                }
            }))
            .absolute()
            .top(look.length(token::H_TITLE_BAR) + look.length(token::SPACE_2))
            .right(look.length(token::SPACE_6))
            .w(gpui::px(WIDTH + 40.))
            .max_h(gpui::px(480.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .font_family(sans(look))
            .child(
                div()
                    .px(look.length(token::SPACE_3))
                    .pb(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_MUTED))
                    .child("Toasts"),
            )
            .children(rows);
        labelled(panel, Role::Dialog, "Toasts")
    }
}

impl Render for Toasts {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let shown: Vec<Toast> = self.shelf.shown().into_iter().cloned().collect();
        let last = shown.len().saturating_sub(1);
        // Fleet's states stand first, at the top of the stack: urgent, and held while they last.
        let fleet: Vec<FleetNotice> = cx.try_global::<FleetNotices>().map(|kept| kept.0.clone()).unwrap_or_default();
        // Under the title bar at most: in a short window the oldest is cut at the top, never the bell.
        let stack = div()
            .absolute()
            .top(look.length(token::H_TITLE_BAR) + look.length(token::SPACE_2))
            .right(look.length(token::SPACE_6))
            .bottom(look.length(token::SPACE_6))
            .flex()
            .flex_col()
            .justify_end()
            .overflow_hidden()
            .gap(look.length(token::SPACE_3))
            .children(fleet.iter().map(|notice| self.fleet_toast(notice, look)))
            .children(shown.iter().enumerate().map(|(at, toast)| self.toast(toast, at == last, look, cx)));
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(stack)
            .when(self.history, |layer| layer.child(self.history_panel(look, cx)))
    }
}

/// The bell in the title bar: the history on a press, a dot while a toast is unseen.
struct Bell {
    toasts: WeakEntity<Toasts>,
    _sub: Subscription,
}

impl Bell {
    fn new(toasts: &Entity<Toasts>, cx: &mut Context<Self>) -> Self {
        Bell { toasts: toasts.downgrade(), _sub: cx.observe(toasts, |_, _, cx| cx.notify()) }
    }
}

impl Render for Bell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let unseen = self.toasts.upgrade().is_some_and(|toasts| toasts.read(cx).unseen);
        let toasts = self.toasts.clone();
        let key = crate::generated::actions::ACTIONS.iter().find(|act| act.id == "toast_history").map(|act| act.shortcut).unwrap_or_default();
        let bell: Stateful<gpui::Div> = icon_button("toast-bell", "bell", look.color(token::FG_MUTED), look)
            .relative()
            .when(unseen, |bell| {
                bell.child(
                    div()
                        .absolute()
                        .right(look.length(token::SPACE_1))
                        .top(look.length(token::SPACE_1))
                        .size(look.length(token::SPACE_2))
                        .rounded_full()
                        .bg(look.color(token::OPERATOR)),
                )
            })
            .tooltip(tip(if unseen { format!("Toasts, one unseen — {key}") } else { format!("Toasts — {key}") }))
            .on_click(move |_, window, cx| {
                if let Some(toasts) = toasts.upgrade() {
                    toasts.update(cx, |toasts, cx| toasts.toggle_history(window, cx));
                }
            });
        labelled(bell, Role::Button, "Toasts")
    }
}

/// The edge: red for urgent, amber for a decision, the accent for a go-ahead.
fn edge_of(urgency: Urgency, look: &Look) -> Hsla {
    look.color(match urgency {
        Urgency::Urgent => token::STATUS_COMPLETED_FAILED,
        Urgency::Decision => token::STATUS_AWAITING_REVIEW,
        Urgency::GoAhead => token::ACCENT,
    })
}

/// What the item asks, where a Job holds it; else the Operator's read.
fn sentence(item: &OperatorTurnItem) -> SharedString {
    face::asks_of(item).or_else(|| item.read.as_ref().map(|read| SharedString::from(read.read.clone()))).unwrap_or_default()
}

/// The muted line: the Operator's read under what a Job asks; else where the item is.
fn muted_line(item: &OperatorTurnItem) -> Option<SharedString> {
    match face::asks_of(item) {
        Some(_) => item.read.as_ref().map(|read| read.read.clone().into()),
        None => item.repository.clone().map(Into::into),
    }
}

/// `14:10`, in this Mac's time.
fn clock_of(at: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(at).map(|at| at.with_timezone(&chrono::Local).format("%H:%M").to_string()).unwrap_or_default()
}

fn history_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Armada/native-toasts"))
}

/// One toast a line: how it ended, a tab, the item as Fleet served it.
fn line_of(toast: &Toast) -> Option<String> {
    let ended = match toast.ended {
        None => "open",
        Some(Ended::Settled) => "settled",
        Some(Ended::Closed) => "closed",
        Some(Ended::Later) => "later",
    };
    ipc::encode(&toast.item).ok().map(|item| format!("{ended}\t{item}"))
}

fn read_history(file: &PathBuf) -> Vec<Toast> {
    let Ok(text) = std::fs::read_to_string(file) else { return Vec::new() };
    text.lines()
        .filter_map(|line| {
            let (ended, item) = line.split_once('\t')?;
            let item = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", item.as_bytes()).ok()?;
            let ended = match ended {
                "settled" => Some(Ended::Settled),
                "closed" => Some(Ended::Closed),
                "later" => Some(Ended::Later),
                _ => None,
            };
            Some(Toast { urgency: item.urgency, item, ended })
        })
        .collect()
}
