//! Deferred, alone in its pane, as Armada Classic's `LaterBlock` draws it flat: each follow-up the
//! agent kept and he has not confirmed, a card in Deferred's hue with Remind and two thumbs in
//! columns of their own. Thumbs up keeps it, thumbs down takes it off the list, and a right press or
//! Shift+F10 on the row offers the same two. A press takes the row away at once; a refusal brings it
//! back and says Fleet's words in a toast.

use gpui::{div, prelude::*, AnyElement, Context, KeyDownEvent, MouseButton, Role, SharedString, Window};
use ipc::{ConfirmLater, LaterHow, SessionId, SettleLater};

use super::head::{below_left, drawn};
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What Remind sends (`remindText`, `cockpit/deck/acts.ts`).
fn remind_text(sentence: &str) -> String {
    format!("You said: \"{sentence}\" Do it, file it, or drop it with a reason.")
}

impl JobView {
    pub(super) fn side_deferred(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(job) = self.record(id, cx) else { return div().into_any_element() };
        let hue = look.color(token::TOOL_LOOK);
        let edge = look.length(token::BORDER_WIDTH);
        let cell = look.length(token::SPACE_6);
        let rows: Vec<AnyElement> = job
            .later
            .iter()
            .filter(|later| !later.confirmed && !self.later_settled(&later.id))
            .map(|later| {
                let (sentence, later_id) = (later.sentence.clone(), later.id.clone());
                let act = |name: String, icon: &'static str, said: &'static str, on: Box<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>| {
                    labelled(
                        div()
                            .id(SharedString::from(name))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(cell)
                            .rounded(look.length(token::RADIUS_MD))
                            .cursor_pointer()
                            .tab_index(0)
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                            .tooltip(tip(said))
                            .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
                            .on_click(cx.listener(move |this, _, window, cx| on(this, window, cx))),
                        Role::Button,
                        format!("{said}: {sentence}"),
                    )
                };
                let remind = sentence.clone();
                let (confirm, drop) = (later_id.clone(), later_id.clone());
                let (menu_at, menu_keys) = ((later_id.clone(), sentence.clone()), (later_id.clone(), sentence.clone()));
                let row = div()
                    .id(SharedString::from(format!("later:{later_id}")))
                    .relative()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .py(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .border(edge)
                    .border_color(look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.4)))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_RAISED).blend(hue.opacity(0.07)))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                            this.open_later_menu(menu_at.0.clone(), event.position, false, window, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        let keys = &event.keystroke;
                        if keys.key == "menu" || (keys.key == "f10" && keys.modifiers.shift) {
                            let (later, sentence) = &menu_keys;
                            if let Some(at) = drawn(window, cx, Role::Button, &format!("Remind: {sentence}")).map(below_left) {
                                this.open_later_menu(later.clone(), at, true, window, cx);
                                cx.stop_propagation();
                            }
                        }
                    }))
                    .child(div().absolute().left_0().top_0().bottom_0().w(edge * 3.).bg(hue))
                    .child(div().flex_1().min_w_0().child(SharedString::from(later.sentence.clone())))
                    .child(div().flex_none().w(cell).child(act(format!("remind:{later_id}"), "bell-ring", "Remind", Box::new(move |this, window, cx| this.send_line(remind_text(&remind), window, cx)))))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .gap(look.length(token::SPACE_1))
                            .w(cell * 2. + look.length(token::SPACE_1))
                            .child(act(format!("confirm:{later_id}"), "thumbs-up", "Deferred", Box::new(move |this, window, cx| this.later_confirm(confirm.clone(), window, cx))))
                            .child(act(format!("drop:{later_id}"), "thumbs-down", "Not deferred", Box::new(move |this, window, cx| this.later_drop(drop.clone(), window, cx)))),
                    );
                labelled(row, Role::Group, later.sentence.clone()).into_any_element()
            })
            .collect();
        if rows.is_empty() {
            return div().into_any_element();
        }
        labelled(div().id("deferred").flex().flex_col().gap(look.length(token::SPACE_1)).children(rows), Role::List, "Deferred").into_any_element()
    }

    /// The row's menu: Deferred and Not deferred, as its thumbs.
    fn open_later_menu(&mut self, later: String, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entries = vec![
            Entry::Act { id: "yes", label: "Deferred".into(), icon: Some("thumbs-up"), danger: false },
            Entry::Act { id: "no", label: "Not deferred".into(), icon: Some("thumbs-down"), danger: false },
        ];
        let back = window.focused(cx);
        self.open_pop(
            "Deferred",
            entries,
            at,
            from_keys,
            back,
            move |this, picked, window, cx| match picked {
                "yes" => this.later_confirm(later.clone(), window, cx),
                _ => this.later_drop(later.clone(), window, cx),
            },
            window,
            cx,
        );
    }

    fn later_settled(&self, later: &str) -> bool {
        self.shown.as_ref().and_then(|id| self.pages.get(id.as_str())).is_some_and(|page| page.side.settled.contains(later))
    }

    /// The pressed row is gone, and the keys with it: they go to the message box, so they still reach the Job.
    fn keys_to_box(&self, window: &mut Window, cx: &mut Context<Self>) {
        let keys = gpui::Focusable::focus_handle(self.composer.input.read(cx), cx);
        window.focus(&keys, cx);
    }

    /// Take the row away now; a refusal puts it back and is said in a toast.
    fn later_settle(&mut self, job: &SharedString, later: &str, settled: bool, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            if settled {
                page.side.settled.insert(later.to_string());
            } else {
                page.side.settled.remove(later);
            }
        }
        cx.notify();
    }

    fn later_confirm(&mut self, later: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        self.later_settle(&job, &later, true, cx);
        self.keys_to_box(window, cx);
        let owner = job.clone();
        let body = ipc::encode(&ConfirmLater { id: later.clone(), session_id: Some(SessionId::carried(job.to_string())) }).map_err(|why| format!("{why:?}"));
        self.post_with(job, "/sessions/later/confirm", body, move |this, why, cx| this.later_refused(&owner, &later, why, cx), window, cx);
    }

    fn later_drop(&mut self, later: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        self.later_settle(&job, &later, true, cx);
        self.keys_to_box(window, cx);
        let owner = job.clone();
        let settle = SettleLater { id: later.clone(), how: LaterHow::Dropped, issue: None, reason: None, session_id: Some(SessionId::carried(job.to_string())), misread: true };
        let body = ipc::encode(&settle).map_err(|why| format!("{why:?}"));
        self.post_with(job, "/sessions/settle_later", body, move |this, why, cx| this.later_refused(&owner, &later, why, cx), window, cx);
    }

    fn later_refused(&mut self, job: &SharedString, later: &str, why: SharedString, cx: &mut Context<Self>) {
        self.later_settle(job, later, false, cx);
        cx.defer(move |cx| crate::toasts::tell(why, cx));
    }
}
